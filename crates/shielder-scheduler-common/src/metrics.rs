use std::collections::HashSet;

use metrics::histogram;
use tokio::time::Instant;
use tracing::{
    span::{Attributes, Id},
    Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

/// A tracing_subscriber layer that collects timing metrics for spans.
///
/// Based on tracing_subscriber::fmt. Handles spans that are entered and exited
/// multiple times, which is needed to track the time spent in busy and idle states
/// for asynchronous operations.
///
/// The metrics are submitted to the metrics crate as histograms.
/// ```rust
/// use shielder_scheduler_common::metrics::{FutureHistogramLayer, span_names};
/// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
///
///     // Option 1: Track all instrumented spans
///     tracing_subscriber::registry()
///         .with(FutureHistogramLayer::new().with_filter(EnvFilter::new("info")))
///         .init();
///
///     // Option 2: Track only specific spans
///     tracing_subscriber::registry()
///         .with(
///             FutureHistogramLayer::with_spans(&[
///                 "my_custom_span",
///                 "another_span",
///                 "database_operation",
///             ])
///             .with_filter(EnvFilter::new("info"))
///         )
///         .init();
/// ```
#[derive(Debug, Clone)]
pub struct FutureHistogramLayer {
    /// Optional set of span names to track. If None, all spans are tracked.
    tracked_spans: Option<HashSet<String>>,
}

impl FutureHistogramLayer {
    /// Create a new layer that tracks all instrumented spans
    pub fn new() -> Self {
        Self {
            tracked_spans: None,
        }
    }

    /// Create a new layer that tracks only the specified span names
    pub fn with_spans<S: AsRef<str>>(spans: &[S]) -> Self {
        let tracked_spans = spans.iter().map(|s| s.as_ref().to_string()).collect();
        Self {
            tracked_spans: Some(tracked_spans),
        }
    }

    /// Check if a span should be tracked
    fn should_track_span(&self, span_name: &str) -> bool {
        match &self.tracked_spans {
            None => true, // Track all spans
            Some(tracked) => tracked.contains(span_name),
        }
    }
}

impl Default for FutureHistogramLayer {
    fn default() -> Self {
        Self::new()
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
        if self.should_track_span(metadata.name()) {
            tracing::subscriber::Interest::always()
        } else {
            tracing::subscriber::Interest::never()
        }
    }

    fn on_new_span(&self, _: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if extensions.get_mut::<Timings>().is_none() {
            extensions.insert(Timings::new());
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
