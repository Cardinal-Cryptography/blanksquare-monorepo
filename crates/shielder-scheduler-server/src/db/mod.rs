use sqlx::postgres::PgPoolOptions;
pub use sqlx::PgPool;
use tracing::info;

use crate::{command_line_args::CommandLineArgs, error::SchedulerServerError as Error};

pub mod schema;
pub use schema::*;

pub async fn connect_to_db(options: &CommandLineArgs) -> Result<PgPool, Error> {
    let ssl_mode = if options.db_ssl { "require" } else { "disable" };

    let db_connection_str = format!(
        "postgres://{}:{}@{}:{}/{}?sslmode={}",
        options.db_user,
        options.db_pass,
        options.db_host,
        options.db_port,
        options.db_name,
        ssl_mode
    );

    info!("Connecting to database...");
    // Create database connection pool
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_connection_str)
        .await
        .map_err(Error::DatabaseError)
}
