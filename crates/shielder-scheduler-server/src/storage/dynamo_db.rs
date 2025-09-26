use std::time::Duration;

use aws_sdk_dynamodb::{
    error::SdkError,
    operation::describe_table::DescribeTableError,
    types::{
        AttributeDefinition, AttributeValue, BillingMode, GlobalSecondaryIndex, IndexStatus,
        KeySchemaElement, KeyType, Projection, ProjectionType, ScalarAttributeType, TableStatus,
    },
    Client,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::info;

use crate::storage::{RequestStatus, ScheduledRequest, StorageError, StorageInterface};

const CHECK_ACTIVE_MAX_ATTEMPTS: usize = 10;
const CHECK_ACTIVE_ATTEMPT_SLEEP_DURATION: Duration = Duration::from_secs(6);

pub struct DynamoDb {
    client: Client,
    table_name: String,
}

impl DynamoDb {
    pub async fn new(rpc_url: &str) -> Result<Self, StorageError> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&config);
        let chain_id = get_chain_id(rpc_url).await?;
        let table_name = format!("scheduled-requests-{}", chain_id);
        let db = Self { client, table_name };
        db.create_table_if_not_exists().await?;
        Ok(db)
    }

    async fn create_table_if_not_exists(&self) -> Result<(), StorageError> {
        // Describe the table. If it exists return Ok
        match self
            .client
            .describe_table()
            .table_name(&self.table_name)
            .send()
            .await
        {
            Ok(_) => return Ok(()),
            Err(e) => {
                if !is_resource_not_found(&e) {
                    return Err(map_internal("describe_table", e));
                }
            }
        }

        // Primary key: last_note_index
        let last_note_index_attr = AttributeDefinition::builder()
            .attribute_name("last_note_index")
            .attribute_type(ScalarAttributeType::S)
            .build()
            .map_err(|e| {
                StorageError::Internal(format!("LastNoteIndex AttrDef build error: {e}"))
            })?;

        let primary_key_schema = KeySchemaElement::builder()
            .attribute_name("last_note_index")
            .key_type(KeyType::Hash)
            .build()
            .map_err(|e| {
                StorageError::Internal(format!("PrimaryKey KeySchema build error: {e}"))
            })?;

        // GSI attributes
        let status_attr = AttributeDefinition::builder()
            .attribute_name("status")
            .attribute_type(ScalarAttributeType::S)
            .build()
            .map_err(|e| StorageError::Internal(format!("Status AttrDef build error: {e}")))?;

        let relay_after_attr = AttributeDefinition::builder()
            .attribute_name("relay_after")
            .attribute_type(ScalarAttributeType::N)
            .build()
            .map_err(|e| StorageError::Internal(format!("RelayAfter AttrDef build error: {e}")))?;

        // GSI definition
        let gsi = GlobalSecondaryIndex::builder()
            .index_name("StatusRelayAfterIndex")
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("status")
                    .key_type(KeyType::Hash)
                    .build()
                    .map_err(|e| {
                        StorageError::Internal(format!("GSI Hash KeySchema build error: {e}"))
                    })?,
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("relay_after")
                    .key_type(KeyType::Range)
                    .build()
                    .map_err(|e| {
                        StorageError::Internal(format!("GSI Range KeySchema build error: {e}"))
                    })?,
            )
            .projection(
                Projection::builder()
                    .projection_type(ProjectionType::All)
                    .build(),
            )
            .build()
            .map_err(|e| StorageError::Internal(format!("GSI build error: {e}")))?;

        if let Err(e) = self
            .client
            .create_table()
            .table_name(&self.table_name)
            .attribute_definitions(last_note_index_attr)
            .attribute_definitions(status_attr)
            .attribute_definitions(relay_after_attr)
            .key_schema(primary_key_schema)
            .global_secondary_indexes(gsi)
            .billing_mode(BillingMode::PayPerRequest)
            .send()
            .await
        {
            return Err(StorageError::Internal(format!(
                "Failed to create DynamoDB table: {e}"
            )));
        }

        // Wait for table + GSIs to become ACTIVE
        info!(
            "Waiting for DynamoDB table {} to become ACTIVE",
            self.table_name
        );
        for attempt in 0..CHECK_ACTIVE_MAX_ATTEMPTS {
            info!(
                "Check attempt {}/{}",
                attempt + 1,
                CHECK_ACTIVE_MAX_ATTEMPTS
            );
            let desc = self
                .client
                .describe_table()
                .table_name(&self.table_name)
                .send()
                .await
                .map_err(|e| map_internal("describe_table(wait)", e))?;

            if let Some(table) = desc.table() {
                let table_ready = matches!(table.table_status(), Some(TableStatus::Active));
                let gsis_ready = table
                    .global_secondary_indexes()
                    .iter()
                    .all(|g| matches!(g.index_status(), Some(IndexStatus::Active)));

                if table_ready && gsis_ready {
                    info!("DynamoDB table and GSIs are ACTIVE");
                    return Ok(());
                }
            }
            tokio::time::sleep(CHECK_ACTIVE_ATTEMPT_SLEEP_DURATION).await;
        }

        Err(StorageError::Internal(
            "Timed out waiting for DynamoDB table / GSIs to become ACTIVE".into(),
        ))
    }

    async fn get_item_by_last_note_index(
        &self,
        last_note_index: &str,
    ) -> Result<Option<ScheduledRequest>, StorageError> {
        let out = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key(
                "last_note_index",
                AttributeValue::S(last_note_index.to_string()),
            )
            .send()
            .await
            .map_err(|e| map_internal("get_item", e))?;

        if let Some(item) = out.item() {
            if let Some(AttributeValue::S(request_attr)) = item.get("request") {
                let req: ScheduledRequest = serde_json::from_str(request_attr).map_err(|e| {
                    StorageError::Internal(format!("Failed to deserialize scheduled request: {e}"))
                })?;
                return Ok(Some(req));
            }
        }
        Ok(None)
    }

    async fn put_request(
        &self,
        request: &ScheduledRequest,
        condition_expression: Option<&str>,
    ) -> Result<(), StorageError> {
        let request_serialized = serde_json::to_string(&request)
            .map_err(|e| StorageError::Internal(format!("Failed to serialize request: {e}")))?;

        let mut builder = self.client.put_item();
        builder = builder
            .table_name(&self.table_name)
            .item(
                "last_note_index",
                AttributeValue::S(request.last_note_index.to_string()),
            )
            .item(
                "status",
                AttributeValue::S(status_to_str(&request.status).to_string()),
            )
            .item(
                "relay_after",
                AttributeValue::N(request.relay_after.timestamp_millis().to_string()),
            )
            .item(
                "created_at",
                AttributeValue::N(request.created_at.timestamp().to_string()),
            )
            .item("request", AttributeValue::S(request_serialized));

        if let Some(condition) = condition_expression {
            builder = builder.condition_expression(condition);
        }

        if let Some(error_msg) = &request.error_message {
            builder = builder.item("error_message", AttributeValue::S(error_msg.clone()));
        }
        if let Some(processed_at) = request.processed_at {
            builder = builder.item(
                "processed_at",
                AttributeValue::N(processed_at.timestamp().to_string()),
            );
        }

        builder
            .send()
            .await
            .map_err(|e| StorageError::Internal(format!("Failed to put item: {e}")))?;
        Ok(())
    }
}

impl StorageInterface for DynamoDb {
    async fn insert_scheduled_request(
        &self,
        request: ScheduledRequest,
    ) -> Result<(), StorageError> {
        match self
            .put_request(&request, Some("attribute_not_exists(last_note_index)"))
            .await
        {
            Ok(()) => Ok(()),
            Err(StorageError::Internal(msg)) if msg.contains("ConditionalCheckFailedException") => {
                Err(StorageError::DuplicateEntry(
                    request.last_note_index.to_string(),
                ))
            }
            Err(e) => Err(e),
        }
    }

    async fn get_pending_requests(
        &self,
        limit: usize,
    ) -> Result<Vec<ScheduledRequest>, StorageError> {
        let now = Utc::now().timestamp_millis().to_string();

        let result = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name("StatusRelayAfterIndex")
            .key_condition_expression("#status = :status AND relay_after <= :max_time")
            .expression_attribute_names("#status", "status")
            .expression_attribute_values(
                ":status",
                AttributeValue::S(status_to_str(&RequestStatus::Pending).to_string()),
            )
            .expression_attribute_values(":max_time", AttributeValue::N(now))
            .limit(limit as i32)
            .send()
            .await
            .map_err(|e| map_internal("query", e))?;

        let mut requests = Vec::new();
        for item in result.items() {
            if let Some(AttributeValue::S(request_attr)) = item.get("request") {
                let req: ScheduledRequest =
                    serde_json::from_str(request_attr.as_str()).map_err(|e| {
                        StorageError::Internal(format!(
                            "Failed to deserialize scheduled request: {e}"
                        ))
                    })?;
                requests.push(req);
            }
        }

        Ok(requests)
    }

    async fn update_request_status(
        &self,
        last_note_index: &str,
        status: RequestStatus,
        processed_at: Option<DateTime<Utc>>,
        error_message: Option<&str>,
    ) -> Result<(), StorageError> {
        let Some(mut existing) = self.get_item_by_last_note_index(last_note_index).await? else {
            return Err(StorageError::NotFound(last_note_index.to_string()));
        };
        existing.status = status;
        existing.error_message = error_message.map(|s| s.to_string());
        existing.processed_at = processed_at;

        // Simple put since last_note_index (primary key) doesn't change
        self.put_request(&existing, None).await
    }

    async fn update_retry_attempt(
        &self,
        last_note_index: &str,
        new_relay_after: DateTime<Utc>,
        new_retry_count: i32,
        processed_at: Option<DateTime<Utc>>,
        new_error_message: Option<&str>,
    ) -> Result<(), StorageError> {
        let Some(mut existing) = self.get_item_by_last_note_index(last_note_index).await? else {
            return Err(StorageError::NotFound(last_note_index.to_string()));
        };
        existing.relay_after = new_relay_after;
        existing.retry_count = new_retry_count;
        existing.error_message = new_error_message.map(|s| s.to_string());
        existing.processed_at = processed_at;

        // Simple put since last_note_index (primary key) doesn't change
        self.put_request(&existing, None).await
    }

    async fn get_request_by_last_note_index(
        &self,
        last_note_index: &str,
    ) -> Result<Option<ScheduledRequest>, StorageError> {
        // Direct lookup by primary key
        self.get_item_by_last_note_index(last_note_index).await
    }
}

fn status_to_str(status: &RequestStatus) -> &'static str {
    match status {
        RequestStatus::Pending => "pending",
        RequestStatus::Completed => "completed",
        RequestStatus::Failed => "failed",
    }
}

fn is_resource_not_found(e: &SdkError<DescribeTableError>) -> bool {
    matches!(e, SdkError::ServiceError(inner) if inner.err().is_resource_not_found_exception())
}

fn map_internal<E: std::fmt::Display>(op: &str, e: SdkError<E>) -> StorageError {
    StorageError::Internal(format!(
        "AWS SDK {} error: {}",
        op,
        e.as_service_error()
            .map(|se| se.to_string())
            .unwrap_or_else(|| e.to_string())
    ))
}

#[derive(Deserialize, Debug)]
struct ChainIdResponse {
    result: String, // This will be a hex string like "0x1"
}

async fn get_chain_id(rpc_url: &str) -> Result<u64, StorageError> {
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_chainId",
        "params": []
    });

    let response: ChainIdResponse = client
        .post(rpc_url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| StorageError::Internal(format!("Failed to send RPC request: {e}")))?
        .json()
        .await
        .map_err(|e| StorageError::Internal(format!("Failed to parse RPC response: {e}")))?;

    let chain_id = u64::from_str_radix(response.result.trim_start_matches("0x"), 16)
        .map_err(|e| StorageError::Internal(format!("Failed to parse chain ID: {e}")))?;

    Ok(chain_id)
}
