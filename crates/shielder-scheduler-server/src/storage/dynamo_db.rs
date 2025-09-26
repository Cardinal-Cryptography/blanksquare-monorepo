use aws_sdk_dynamodb::{
    error::SdkError,
    operation::describe_table::DescribeTableError,
    types::{AttributeDefinition, AttributeValue, KeySchemaElement, KeyType, ScalarAttributeType},
    Client,
};
use chrono::{DateTime, Utc};

use crate::storage::{RequestStatus, ScheduledRequest, StorageError, StorageInterface};

pub struct DynamoDb {
    client: Client,
    pending_requests_table_name: String,
    completed_requests_table_name: String,
}

impl DynamoDb {
    pub async fn new() -> Result<Self, StorageError> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&config);
        // TODO: Replace <chain-id> with actual chain ID from config
        let pending_requests_table_name = "pending-requests-<chain-id>";
        let completed_requests_table_name = "completed-requests-<chain-id>";
        let db = Self {
            client,
            pending_requests_table_name: pending_requests_table_name.to_string(),
            completed_requests_table_name: completed_requests_table_name.to_string(),
        };
        db.create_table_if_not_exists(&pending_requests_table_name)
            .await?;
        db.create_table_if_not_exists(&completed_requests_table_name)
            .await?;
        Ok(db)
    }

    async fn create_table_if_not_exists(&self, table_name: &str) -> Result<(), StorageError> {
        // Describe the table. If it exists return Ok
        match self
            .client
            .describe_table()
            .table_name(table_name)
            .send()
            .await
        {
            // If the table exists, return Ok
            Ok(_) => return Ok(()),
            // If the error is not ResourceNotFoundException, return the error
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

        let relay_after_attr = AttributeDefinition::builder()
            .attribute_name("relay_after")
            .attribute_type(ScalarAttributeType::N)
            .build()
            .map_err(|e| StorageError::Internal(format!("RelayAfter AttrDef build error: {e}")))?;

        let partition_key_schema = KeySchemaElement::builder()
            .attribute_name("last_note_index")
            .key_type(KeyType::Hash)
            .build()
            .map_err(|e| {
                StorageError::Internal(format!("PrimaryKey KeySchema build error: {e}"))
            })?;

        let relay_after_schema = KeySchemaElement::builder()
            .attribute_name("relay_after")
            .key_type(KeyType::Range)
            .build()
            .map_err(|e| {
                StorageError::Internal(format!("RelayAfter KeySchema build error: {e}"))
            })?;

        if let Err(e) = self
            .client
            .create_table()
            .table_name(table_name)
            .attribute_definitions(last_note_index_attr)
            .attribute_definitions(relay_after_attr)
            .key_schema(partition_key_schema)
            .key_schema(relay_after_schema)
            .send()
            .await
        {
            return Err(StorageError::Internal(format!(
                "Failed to create DynamoDB table: {e}"
            )));
        }

        Ok(())
    }

    async fn get_item(
        &self,
        table_name: &str,
        last_note_index: &str,
    ) -> Result<Option<ScheduledRequest>, StorageError> {
        let out = self
            .client
            .get_item()
            .table_name(table_name)
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

    async fn delete_item(
        &self,
        table_name: &str,
        last_note_index: &str,
    ) -> Result<(), StorageError> {
        self.client
            .delete_item()
            .table_name(table_name)
            .key(
                "last_note_index",
                AttributeValue::S(last_note_index.to_string()),
            )
            .send()
            .await
            .map_err(|e| map_internal("delete_item", e))?;
        Ok(())
    }

    async fn put_request(
        &self,
        table_name: &str,
        request: &ScheduledRequest,
        condition_expression: Option<&str>,
    ) -> Result<(), StorageError> {
        let request_string = serde_json::to_string(&request).map_err(|e| {
            StorageError::Internal(format!("Failed to serialize request request: {e}"))
        })?;

        let mut builder = self.client.put_item();
        if let Some(condition) = condition_expression {
            builder = builder.condition_expression(condition);
        }
        builder = builder
            .table_name(table_name)
            .item(
                "last_note_index",
                AttributeValue::S(request.last_note_index.to_string()),
            )
            .item(
                "relay_after",
                AttributeValue::N(request.relay_after.timestamp_millis().to_string()),
            )
            .item("request", AttributeValue::S(request_string));

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
            .put_request(
                &self.pending_requests_table_name,
                &request,
                Some("attribute_not_exists(last_note_index)"),
            )
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
            .table_name(&self.pending_requests_table_name)
            .index_name("StatusRelayAfterIndex")
            .key_condition_expression("relay_after <= :max_time")
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
        error_message: Option<&str>,
    ) -> Result<(), StorageError> {
        let (from_table_name, to_table_name) = match status {
            RequestStatus::Completed | RequestStatus::Failed => (
                &self.pending_requests_table_name,
                &self.completed_requests_table_name,
            ),
            RequestStatus::Pending => (
                &self.completed_requests_table_name,
                &self.pending_requests_table_name,
            ),
        };

        let Some(mut existing) = self.get_item(from_table_name, last_note_index).await? else {
            return Err(StorageError::NotFound(last_note_index.to_string()));
        };

        existing.status = status;
        existing.error_message = error_message.map(|s| s.to_string());
        existing.processed_at = Some(Utc::now());

        self.put_request(to_table_name, &existing, None).await?;
        self.delete_item(from_table_name, last_note_index).await
    }

    async fn update_retry_attempt(
        &self,
        last_note_index: &str,
        new_relay_after: DateTime<Utc>,
        new_retry_count: i32,
        new_error_message: Option<&str>,
    ) -> Result<(), StorageError> {
        let Some(mut existing) = self
            .get_item(&self.pending_requests_table_name, last_note_index)
            .await?
        else {
            return Err(StorageError::NotFound(last_note_index.to_string()));
        };
        existing.relay_after = new_relay_after;
        existing.retry_count = new_retry_count;
        existing.error_message = new_error_message.map(|s| s.to_string());

        self.put_request(&self.pending_requests_table_name, &existing, None)
            .await
    }

    async fn get_request_by_last_note_index(
        &self,
        last_note_index: &str,
    ) -> Result<Option<ScheduledRequest>, StorageError> {
        match self
            .get_item(&self.pending_requests_table_name, last_note_index)
            .await?
        {
            Some(req) => Ok(Some(req)),
            None => {
                self.get_item(&self.completed_requests_table_name, last_note_index)
                    .await
            }
        }
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
