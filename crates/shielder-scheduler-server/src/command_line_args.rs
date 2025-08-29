use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub struct CommandLineArgs {
    // Server configuration
    /// A port on which this server listens to incoming HTTP connections.
    #[clap(short, long, default_value = "3000", env = "PUBLIC_PORT")]
    pub public_port: u16,

    /// A port on which this server exposes its metrics endpoint.
    #[clap(short, long, default_value = "3001", env = "METRICS_PORT")]
    pub metrics_port: u16,

    /// Local IPv4 address on which this server listens to incoming HTTP connections
    #[clap(short, long, default_value = "0.0.0.0", env = "BIND_ADDRESS")]
    pub bind_address: String,

    /// Maximum request size (in bytes) sent to server
    #[clap(long, default_value_t = 100 * 1024, env = "MAXIMUM_REQUEST_SIZE")]
    pub maximum_request_size: usize,

    // Metrics configuration
    /// How often to perform metric upkeep
    #[clap(long, default_value_t = 60, env = "METRICS_UPKEEP_TIMEOUT_SECS")]
    pub(crate) metrics_upkeep_timeout_secs: u64,

    /// How long are the buckets from which metric histograms are built
    #[clap(long, default_value_t = 60, env = "METRICS_BUCKET_DURATION_SECS")]
    pub(crate) metrics_bucket_duration_secs: u64,

    // TEE configuration
    /// Internal port on which host and tee applications talks to each other
    /// This is the part of the vsock endpoint, which is tee_cid:tee_port
    #[clap(short, long, default_value_t = shielder_scheduler_common::protocol::VSOCK_PORT, env = "TEE_PORT")]
    pub tee_port: u16,

    /// A context identifier on which this server and TEE server communicate with each other
    /// This is the part of the vsock endpoint, which is tee_cid:tee_port
    #[clap(long, default_value_t = vsock::VMADDR_CID_HOST, env = "TEE_CID")]
    pub tee_cid: u32,

    /// How many tasks can be processed in parallel by the TEE task pool
    /// Do not raise it above 128 as this is the limit of vsock connections, at least
    /// for the rust lib used by this server
    #[clap(long, default_value_t = 100, env = "TEE_TASK_POOL_CAPACITY")]
    pub tee_task_pool_capacity: usize,

    /// How much time this server waits for a task to be processed by the TEE task pool
    #[clap(long, default_value_t = 5, env = "TEE_TASK_POOL_TIMEOUT_SECS")]
    pub tee_task_pool_timeout_secs: u64,

    /// How much time this server waits for a response from TEE
    #[clap(long, default_value_t = 60, env = "TEE_COMPUTE_TIMEOUT_SECS")]
    pub tee_compute_timeout_secs: u64,

    // Scheduler processor configuration
    /// How often the scheduler processor checks for new tasks to process
    #[clap(long, default_value_t = 5, env = "SCHEDULER_INTERVAL_SECS")]
    pub scheduler_interval_secs: u64,
    /// How many requests to process in a single batch
    #[clap(long, default_value_t = 10, env = "SCHEDULER_BATCH_SIZE")]
    pub scheduler_batch_size: usize,
    /// How many times a request can be retried before it is removed from the database
    #[clap(long, default_value_t = 3, env = "SCHEDULER_MAX_RETRY_COUNT")]
    pub scheduler_max_retry_count: usize,
    /// How long to wait before retrying a failed request
    #[clap(long, default_value_t = 60, env = "SCHEDULER_RETRY_DELAY_SECS")]
    pub scheduler_retry_delay_secs: u64,

    // Database connection parameters
    /// Database host
    #[clap(long, default_value = "localhost", env = "DB_HOST")]
    pub db_host: String,
    /// Database port
    #[clap(long, default_value = "5432", env = "DB_PORT")]
    pub db_port: u16,
    /// Database name
    #[clap(long, default_value = "scheduler-db", env = "DB_NAME")]
    pub db_name: String,
    /// Database user
    #[clap(long, default_value = "postgres", env = "DB_USER")]
    pub db_user: String,
    /// Database password
    #[clap(long, default_value = "postgres", env = "DB_PASS")]
    pub db_pass: String,
    /// Use SSL for database connection
    /// If set to true, the server will use SSL to connect to the database
    #[clap(long, default_value_t = false, env = "DB_USE_SSL")]
    pub db_ssl: bool,
}
