use alloy_primitives::U256;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shielder_scheduler_common::protocol::EncryptionEnvelope;
use sqlx::{PgPool, Row};

use crate::error::SchedulerServerError as Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledRequest {
    pub id: i64,
    pub encryption_envelope: EncryptionEnvelope,
    pub last_note_index: String, // U256 as string for PostgreSQL
    pub max_relayer_fee: String, // U256 as string for PostgreSQL
    pub relay_after: DateTime<Utc>,
    pub status: RequestStatus,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub retry_count: i32,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "request_status", rename_all = "lowercase")]
pub enum RequestStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl ScheduledRequest {
    pub fn last_note_index_as_u256(&self) -> Result<U256, alloy_primitives::ruint::ParseError> {
        U256::from_str_radix(&self.last_note_index, 10)
    }

    pub fn max_relayer_fee_as_u256(&self) -> Result<U256, alloy_primitives::ruint::ParseError> {
        U256::from_str_radix(&self.max_relayer_fee, 10)
    }
}

pub async fn create_tables(pool: &PgPool) -> Result<(), Error> {
    // Create the request_status enum type
    sqlx::query(
        r#"
        DO $$ 
        BEGIN
            CREATE TYPE request_status AS ENUM (
                'pending', 
                'processing', 
                'completed', 
                'failed'
            );
        EXCEPTION
            WHEN duplicate_object THEN null;
        END $$;
        "#,
    )
    .execute(pool)
    .await?;

    // Create the scheduled_requests table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS scheduled_requests (
            id BIGSERIAL PRIMARY KEY,
            encrypted_payload BYTEA NOT NULL,
            encrypted_dek BYTEA NOT NULL,
            iv BYTEA NOT NULL,
            auth_tag BYTEA NOT NULL,
            last_note_index TEXT NOT NULL,
            max_relayer_fee TEXT NOT NULL,
            relay_after TIMESTAMPTZ NOT NULL,
            status request_status NOT NULL DEFAULT 'pending',
            created_at TIMESTAMPTZ NOT NULL DEFAULT &1,
            processed_at TIMESTAMPTZ,
            retry_count INTEGER NOT NULL DEFAULT 0,
            error_message TEXT
        )
        "#,
    )
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_scheduled_request(
    pool: &PgPool,
    encryption_envelope: EncryptionEnvelope,
    last_note_index: U256,
    max_relayer_fee: U256,
    relay_after: DateTime<Utc>,
) -> Result<i64, Error> {
    let row = sqlx::query(
        r#"
        INSERT INTO scheduled_requests (encrypted_payload, encrypted_dek, iv, auth_tag, last_note_index, max_relayer_fee, relay_after)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
    )
    .bind(encryption_envelope.encrypted_payload)
    .bind(encryption_envelope.encrypted_dek)
    .bind(encryption_envelope.iv)
    .bind(encryption_envelope.auth_tag)
    .bind(last_note_index.to_string())
    .bind(max_relayer_fee.to_string())
    .bind(relay_after)
    .fetch_one(pool)
    .await?;

    Ok(row.get("id"))
}

pub async fn get_pending_requests(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<ScheduledRequest>, Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, encrypted_payload, encrypted_dek, iv, auth_tag, last_note_index, max_relayer_fee, relay_after, 
               status, created_at, processed_at, retry_count, error_message
        FROM scheduled_requests
        WHERE status IN ('pending', 'processing') 
        AND relay_after <= $2
        ORDER BY relay_after ASC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .bind(Utc::now())
    .fetch_all(pool)
    .await?;

    let mut requests = Vec::new();
    for row in rows {
        requests.push(ScheduledRequest {
            id: row.get("id"),
            encryption_envelope: EncryptionEnvelope {
                encrypted_payload: row.get("encrypted_payload"),
                encrypted_dek: row.get("encrypted_dek"),
                iv: row.get("iv"),
                auth_tag: row.get("auth_tag"),
            },
            last_note_index: row.get("last_note_index"),
            max_relayer_fee: row.get("max_relayer_fee"),
            relay_after: row.get("relay_after"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            processed_at: row.get("processed_at"),
            retry_count: row.get("retry_count"),
            error_message: row.get("error_message"),
        });
    }

    Ok(requests)
}

pub async fn update_request_status(
    pool: &PgPool,
    id: i64,
    status: RequestStatus,
    error_message: Option<&str>,
) -> Result<(), Error> {
    let processed_at = match status {
        RequestStatus::Completed | RequestStatus::Failed | RequestStatus::Processing => {
            Some(Utc::now())
        }
        _ => None,
    };

    sqlx::query(
        r#"
        UPDATE scheduled_requests 
        SET status = $1, processed_at = $2, error_message = $3
        WHERE id = $4
        "#,
    )
    .bind(status)
    .bind(processed_at)
    .bind(error_message)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_retry_attempt(
    pool: &PgPool,
    id: i64,
    new_relay_after: DateTime<Utc>,
    new_error_message: Option<&str>,
) -> Result<(), Error> {
    sqlx::query(
        r#"
        UPDATE scheduled_requests 
        SET retry_count = retry_count + 1, relay_after = $2, status = $3, error_message = $4, processed_at = $5
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(new_relay_after)
    .bind(RequestStatus::Processing)
    .bind(new_error_message)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}
