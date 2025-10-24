use std::{collections::HashSet, net::SocketAddr, time::Duration};

use metrics::{counter, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use tokio::time::Instant;
use tracing::{
    span::{Attributes, Id},
    Subscriber,
};
use tracing_subscriber::{
    fmt,
    layer::{Context, SubscriberExt},
    registry::LookupSpan,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

use crate::error::SchedulerServerError as Error;

mod counters {
    pub(crate) const HEALTH_REQUESTS_TOTAL: &str = "health_requests_total";
    pub(crate) const TEE_PUBLIC_KEY_REQUESTS_TOTAL: &str = "tee_public_key_requests_total";
    pub(crate) const GET_STATUS_REQUESTS_TOTAL: &str = "get_status_requests_total";
    pub(crate) const SCHEDULE_WITHDRAW_REQUESTS_TOTAL: &str = "schedule_withdraw_requests_total";
    pub(crate) const PROCESS_SCHEDULED_REQUESTS_SUCCESS_TOTAL: &str =
        "process_scheduled_requests_success_total";
    pub(crate) const PROCESS_SCHEDULED_REQUESTS_FAILURE_TOTAL: &str =
        "process_scheduled_requests_failure_total";
    pub(crate) const PROCESS_SCHEDULED_REQUESTS_RETRY_TOTAL: &str =
        "process_scheduled_requests_retry_total";
}

fn init_counters() {
    counter!(counters::HEALTH_REQUESTS_TOTAL).increment(0);
    counter!(counters::TEE_PUBLIC_KEY_REQUESTS_TOTAL).increment(0);
    counter!(counters::GET_STATUS_REQUESTS_TOTAL).increment(0);
    counter!(counters::SCHEDULE_WITHDRAW_REQUESTS_TOTAL).increment(0);
    counter!(counters::PROCESS_SCHEDULED_REQUESTS_SUCCESS_TOTAL).increment(0);
    counter!(counters::PROCESS_SCHEDULED_REQUESTS_FAILURE_TOTAL).increment(0);
    counter!(counters::PROCESS_SCHEDULED_REQUESTS_RETRY_TOTAL).increment(0);
}

pub struct Metrics;

impl Metrics {
    pub fn start_metrics_server(
        bind_address: &str,
        metrics_port: u16,
        bucket_duration_secs: u64,
        upkeep_timeout_secs: u64,
    ) -> Result<(), Error> {
        _ = tracing_subscriber::registry()
            .with(fmt::layer().with_filter(EnvFilter::from_default_env()))
            .with(FutureHistogramLayer::with_all_spans().with_filter(EnvFilter::new("info")))
            .try_init();

        let addr: SocketAddr = format!("{}:{}", bind_address, metrics_port)
            .parse()
            .map_err(|_| Error::ParseError("Invalid bind address or port".to_string()))?;
        PrometheusBuilder::new()
            .with_http_listener(addr)
            .set_bucket_duration(Duration::from_secs(bucket_duration_secs))?
            .upkeep_timeout(Duration::from_secs(upkeep_timeout_secs))
            .install()?;
        init_counters();
        Ok(())
    }

    pub fn record_health_request() {
        counter!(counters::HEALTH_REQUESTS_TOTAL).increment(1);
    }

    pub fn record_tee_public_key_request() {
        counter!(counters::TEE_PUBLIC_KEY_REQUESTS_TOTAL).increment(1);
    }

    pub fn record_get_status_request() {
        counter!(counters::GET_STATUS_REQUESTS_TOTAL).increment(1);
    }

    pub fn record_schedule_withdraw_request() {
        counter!(counters::SCHEDULE_WITHDRAW_REQUESTS_TOTAL).increment(1);
    }

    pub fn record_process_scheduled_request_success() {
        counter!(counters::PROCESS_SCHEDULED_REQUESTS_SUCCESS_TOTAL).increment(1);
    }

    pub fn record_process_scheduled_request_failure() {
        counter!(counters::PROCESS_SCHEDULED_REQUESTS_FAILURE_TOTAL).increment(1);
    }

    pub fn record_process_scheduled_request_retry() {
        counter!(counters::PROCESS_SCHEDULED_REQUESTS_RETRY_TOTAL).increment(1);
    }
}

#[derive(Debug, Clone)]
pub enum TrackedSpans {
    All,
    Specific(HashSet<&'static str>),
}

impl TrackedSpans {
    pub fn contains(&self, span_name: &str) -> bool {
        match self {
            TrackedSpans::All => true,
            TrackedSpans::Specific(tracked) => tracked.contains(span_name),
        }
    }
}

impl From<&[&'static str]> for TrackedSpans {
    fn from(spans: &[&'static str]) -> Self {
        TrackedSpans::Specific(spans.iter().copied().collect())
    }
}

/// A tracing_subscriber layer that collects timing metrics for spans.
///
/// Based on tracing_subscriber::fmt. Handles spans that are entered and exited
/// multiple times, which is needed to track the time spent in busy and idle states
/// for asynchronous operations.
///
/// The metrics are submitted to the metrics crate as histograms.
///
/// ## Example usage:
/// ```
/// # use shielder_scheduler_common::metrics::{FutureHistogramLayer, TrackedSpans};
/// # use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
///
///     // Option 1: Track all instrumented spans
///     tracing_subscriber::registry()
///         .with(FutureHistogramLayer::with_all_spans())
///         .init();
/// ```
/// ```
/// # use shielder_scheduler_common::metrics::{FutureHistogramLayer, TrackedSpans};
/// # use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
///     // Option 2: Track only specific spans
///     tracing_subscriber::registry()
///        .with(FutureHistogramLayer::with_specific_spans(&[
///            "specific_span_name_1",
///            "specific_span_name_2",
///        ]))
///        .init();
/// ```
#[derive(Debug, Clone)]
pub struct FutureHistogramLayer {
    /// Optional set of span names to track. If None, all spans are tracked.
    tracked_spans: TrackedSpans,
}

impl FutureHistogramLayer {
    pub fn new(tracked_spans: TrackedSpans) -> Self {
        Self { tracked_spans }
    }

    /// Create a new layer that tracks all instrumented spans.
    ///
    /// Equivalent to
    /// ```
    /// # use shielder_scheduler_common::metrics::{FutureHistogramLayer, TrackedSpans};
    /// FutureHistogramLayer::new(TrackedSpans::All);
    /// ```
    pub fn with_all_spans() -> Self {
        Self::new(TrackedSpans::All)
    }

    /// Create a new layer that tracks only specific spans.
    ///
    /// Equivalent to
    /// ```
    /// # use shielder_scheduler_common::metrics::{FutureHistogramLayer, TrackedSpans};
    /// FutureHistogramLayer::new(TrackedSpans::Specific([ "specific_span_name_1", "specific_span_name_2"].into()));
    /// ```
    pub fn with_specific_spans(spans: &[&'static str]) -> Self {
        Self::new(TrackedSpans::from(spans))
    }

    /// Check if a span should be tracked
    fn is_span_tracked(&self, span_name: &str) -> bool {
        self.tracked_spans.contains(span_name)
    }
}

struct Timings {
    idle: u64,
    busy: u64,
    last: Instant,
}

impl Timings {
    fn new() -> Self {
        Self {
            idle: 0,
            busy: 0,
            last: Instant::now(),
        }
    }
}

impl<S> Layer<S> for FutureHistogramLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn register_callsite(
        &self,
        metadata: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        if self.is_span_tracked(metadata.name()) {
            tracing::subscriber::Interest::always()
        } else {
            tracing::subscriber::Interest::never()
        }
    }

    fn on_new_span(&self, _: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        if self.is_span_tracked(span.metadata().name()) {
            let mut extensions = span.extensions_mut();
            if extensions.get_mut::<Timings>().is_none() {
                extensions.insert(Timings::new());
            }
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();
        if let Some(timings) = extensions.get_mut::<Timings>() {
            let now = Instant::now();
            timings.idle += (now - timings.last).as_micros() as u64;
            timings.last = now;
        }
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();
        if let Some(timings) = extensions.get_mut::<Timings>() {
            let now = Instant::now();
            timings.busy += (now - timings.last).as_micros() as u64;
            timings.last = now;
        }
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).expect("Span not found, this is a bug");
        let extensions = span.extensions();
        if let Some(timing) = extensions.get::<Timings>() {
            let Timings {
                busy,
                mut idle,
                last,
            } = *timing;
            idle += (Instant::now() - last).as_micros() as u64;

            let span_name = span.metadata().name();

            // Record busy histogram
            let busy_histogram = histogram!(format!("{}_busy", span_name));
            busy_histogram.record(micros_to_secs(busy));

            // Record idle histogram
            let idle_histogram = histogram!(format!("{}_idle", span_name));
            idle_histogram.record(micros_to_secs(idle));
        }
    }
}

fn micros_to_secs(micros: u64) -> f64 {
    micros as f64 / 1_000_000.0
}
