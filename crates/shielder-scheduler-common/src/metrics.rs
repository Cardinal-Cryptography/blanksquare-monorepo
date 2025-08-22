use std::collections::HashSet;

use metrics::histogram;
use tokio::time::Instant;
use tracing::{
    span::{Attributes, Id},
    Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

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
///        .with(FutureHistogramLayer::with_specific_spans([
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
