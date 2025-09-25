use aws_sdk_dynamodb::{
    error::SdkError,
    operation::{describe_table::DescribeTableError, put_item::PutItemError},
    types::{
        AttributeDefinition, AttributeValue, KeySchemaElement, KeyType, ProvisionedThroughput,
        ScalarAttributeType,
    },
    Client,
};
use chrono::{DateTime, Utc};

use crate::storage::{RequestStatus, ScheduledRequest, StorageError, StorageInterface};

const TABLE_NAME: &str = "ScheduledRequests";

pub struct DynamoDb {
    client: Client,
}

impl DynamoDb {
    pub async fn new() -> Result<Self, StorageError> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&config);
        let db = Self { client };
        db.create_table_if_not_exists().await?;
        Ok(db)
    }

    async fn create_table_if_not_exists(&self) -> Result<(), StorageError> {
        // Describe the table. If it exists return Ok
        match self
            .client
            .describe_table()
            .table_name(TABLE_NAME)
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

        let attr_def = AttributeDefinition::builder()
            .attribute_name("id")
            .attribute_type(ScalarAttributeType::S)
            .build()
            .map_err(|e| StorageError::Internal(format!("AttrDef build error: {e}")))?;
        let key_schema = KeySchemaElement::builder()
            .attribute_name("id")
            .key_type(KeyType::Hash)
            .build()
            .map_err(|e| StorageError::Internal(format!("KeySchema build error: {e}")))?;
        let throughput = ProvisionedThroughput::builder()
            .read_capacity_units(5)
            .write_capacity_units(5)
            .build()
            .map_err(|e| StorageError::Internal(format!("Throughput build error: {e}")))?;

        if let Err(e) = self
            .client
            .create_table()
            .table_name(TABLE_NAME)
            .attribute_definitions(attr_def)
            .key_schema(key_schema)
            .provisioned_throughput(throughput)
            .send()
            .await
        {
            return Err(StorageError::Internal(format!(
                "Failed to create DynamoDB table: {e}"
            )));
        }

        Ok(())
    }

    async fn get_item_by_id(&self, id: u128) -> Result<Option<ScheduledRequest>, StorageError> {
        let id_str = id.to_string();
        let out = self
            .client
            .get_item()
            .table_name(TABLE_NAME)
            .key("id", AttributeValue::S(id_str))
            .send()
            .await
            .map_err(|e| map_internal("get_item", e))?;

        if let Some(item) = out.item() {
            if let Some(AttributeValue::S(payload_attr)) = item.get("payload") {
                let req: ScheduledRequest = serde_json::from_str(payload_attr).map_err(|e| {
                    StorageError::Internal(format!(
                        "Failed to deserialize scheduled request payload: {e}"
                    ))
                })?;
                return Ok(Some(req));
            }
        }
        Ok(None)
    }

    async fn put_full_request(&self, request: &ScheduledRequest) -> Result<(), StorageError> {
        let payload = serde_json::to_string(request).map_err(|e| {
            StorageError::Internal(format!("Failed to serialize request payload: {e}"))
        })?;

        let mut builder = self.client.put_item();
        builder = builder
            .table_name(TABLE_NAME)
            .item("id", AttributeValue::S(request.id.to_string()))
            .item(
                "status",
                AttributeValue::S(status_to_str(&request.status).to_string()),
            )
            .item(
                "relay_after",
                AttributeValue::N(request.relay_after.timestamp().to_string()),
            )
            .item(
                "created_at",
                AttributeValue::N(request.created_at.timestamp().to_string()),
            )
            .item(
                "last_note_index",
                AttributeValue::S(request.last_note_index.to_string()),
            )
            .item("payload", AttributeValue::S(payload));

        if request.error_message.is_some() {
            builder = builder.item(
                "error_message",
                AttributeValue::S(request.error_message.clone().unwrap()),
            );
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
        let payload = serde_json::to_string(&request)
            .map_err(|e| StorageError::Internal(format!("Failed to serialize request: {e}")))?;

        let mut builder = self
            .client
            .put_item()
            .table_name(TABLE_NAME)
            .condition_expression("attribute_not_exists(id)")
            .item("id", AttributeValue::S(request.id.to_string()))
            .item(
                "status",
                AttributeValue::S(status_to_str(&request.status).to_string()),
            )
            .item(
                "relay_after",
                AttributeValue::N(request.relay_after.timestamp().to_string()),
            )
            .item(
                "created_at",
                AttributeValue::N(request.created_at.timestamp().to_string()),
            )
            .item(
                "last_note_index",
                AttributeValue::S(request.last_note_index.to_string()),
            )
            .item("payload", AttributeValue::S(payload));

        if let Some(err) = &request.error_message {
            builder = builder.item("error_message", AttributeValue::S(err.clone()));
        }
        if let Some(processed_at) = request.processed_at {
            builder = builder.item(
                "processed_at",
                AttributeValue::N(processed_at.timestamp().to_string()),
            );
        }

        match builder.send().await {
            Ok(_) => Ok(()),
            Err(e) => {
                if is_conditional_check_failed(&e) {
                    Err(StorageError::DuplicateEntry(request.id))
                } else {
                    Err(map_internal("put_item", e))
                }
            }
        }
    }

    async fn get_pending_requests(
        &self,
        limit: usize,
    ) -> Result<Vec<ScheduledRequest>, StorageError> {
        // Scan for pending or processing requests whose relay_after is <= now
        let now = Utc::now().timestamp().to_string();
        let out_res = self
            .client
            .scan()
            .table_name(TABLE_NAME)
            .filter_expression("#status IN (:pending, :processing) AND relay_after <= :now")
            .expression_attribute_names("#status", "status")
            .expression_attribute_values(
                ":pending",
                AttributeValue::S(status_to_str(&RequestStatus::Pending).to_string()),
            )
            .expression_attribute_values(
                ":processing",
                AttributeValue::S(status_to_str(&RequestStatus::Processing).to_string()),
            )
            .expression_attribute_values(":now", AttributeValue::N(now))
            .send()
            .await;
        let out = match out_res {
            Ok(o) => o,
            Err(e) => return Err(map_internal("scan", e)),
        };

        let mut requests: Vec<ScheduledRequest> = Vec::new();
        for item in out.items() {
            if let Some(AttributeValue::S(payload_str)) = item.get("payload") {
                match serde_json::from_str::<ScheduledRequest>(payload_str.as_str()) {
                    Ok(req) => requests.push(req),
                    Err(_e) => { /* skip malformed */ }
                }
            }
        }
        // Sort by relay_after then take limit
        requests.sort_by(|a, b| a.relay_after.cmp(&b.relay_after));
        Ok(requests.into_iter().take(limit).collect())
    }

    async fn update_request_status(
        &self,
        id: u128,
        status: RequestStatus,
        error_message: Option<&str>,
    ) -> Result<(), StorageError> {
        let Some(mut existing) = self.get_item_by_id(id).await? else {
            return Err(StorageError::NotFound(id.to_string()));
        };
        existing.status = status;
        existing.error_message = error_message.map(|s| s.to_string());
        let payload = serde_json::to_string(&existing).map_err(|e| {
            StorageError::Internal(format!("Failed to serialize updated request: {e}"))
        })?;

        let mut builder = self
            .client
            .put_item()
            .table_name(TABLE_NAME)
            .condition_expression("attribute_exists(id)")
            .item("id", AttributeValue::S(existing.id.to_string()))
            .item(
                "status",
                AttributeValue::S(status_to_str(&existing.status).to_string()),
            )
            .item(
                "relay_after",
                AttributeValue::N(existing.relay_after.timestamp().to_string()),
            )
            .item(
                "created_at",
                AttributeValue::N(existing.created_at.timestamp().to_string()),
            )
            .item(
                "last_note_index",
                AttributeValue::S(existing.last_note_index.to_string()),
            )
            .item("payload", AttributeValue::S(payload));

        if let Some(err) = &existing.error_message {
            builder = builder.item("error_message", AttributeValue::S(err.clone()));
        }
        if let Some(processed_at) = existing.processed_at {
            builder = builder.item(
                "processed_at",
                AttributeValue::N(processed_at.timestamp().to_string()),
            );
        }

        match builder.send().await {
            Ok(_) => Ok(()),
            Err(e) => {
                if is_conditional_check_failed(&e) {
                    Err(StorageError::NotFound(id.to_string()))
                } else {
                    Err(map_internal("put_item", e))
                }
            }
        }
    }

    async fn update_retry_attempt(
        &self,
        id: u128,
        new_relay_after: DateTime<Utc>,
        new_retry_count: i32,
        new_error_message: Option<&str>,
    ) -> Result<(), StorageError> {
        let Some(mut existing) = self.get_item_by_id(id).await? else {
            return Err(StorageError::NotFound(id.to_string()));
        };
        existing.relay_after = new_relay_after;
        existing.retry_count = new_retry_count;
        existing.error_message = new_error_message.map(|s| s.to_string());
        self.put_full_request(&existing).await
    }

    async fn get_request_by_last_note_index(
        &self,
        last_note_index: &str,
    ) -> Result<Option<ScheduledRequest>, StorageError> {
        let out = self
            .client
            .scan()
            .table_name(TABLE_NAME)
            .filter_expression("#lni = :lni")
            .expression_attribute_names("#lni", "last_note_index")
            .expression_attribute_values(":lni", AttributeValue::S(last_note_index.to_string()))
            .send()
            .await
            .map_err(|e| map_internal("scan", e))?;

        let mut matches: Vec<ScheduledRequest> = Vec::new();
        for item in out.items() {
            if let Some(AttributeValue::S(payload_str)) = item.get("payload") {
                if let Ok(req) = serde_json::from_str::<ScheduledRequest>(payload_str.as_str()) {
                    matches.push(req);
                }
            }
        }
        if matches.is_empty() {
            return Ok(None);
        }
        matches.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(matches.into_iter().next())
    }
}

fn status_to_str(status: &RequestStatus) -> &'static str {
    match status {
        RequestStatus::Pending => "pending",
        RequestStatus::Processing => "processing",
        RequestStatus::Completed => "completed",
        RequestStatus::Failed => "failed",
    }
}

fn is_resource_not_found(e: &SdkError<DescribeTableError>) -> bool {
    matches!(e, SdkError::ServiceError(inner) if inner.err().is_resource_not_found_exception())
}

fn is_conditional_check_failed(e: &SdkError<PutItemError>) -> bool {
    matches!(e, SdkError::ServiceError(inner) if inner.err().is_conditional_check_failed_exception())
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
