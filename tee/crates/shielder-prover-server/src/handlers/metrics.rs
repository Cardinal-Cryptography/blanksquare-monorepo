use enum_map::{enum_map, Enum, EnumMap};
use lazy_static::lazy_static;
use metrics::{histogram, Histogram};
use strum::{EnumIter, EnumString, IntoStaticStr, IntoEnumIterator as _};
use tokio::time::Instant;
use tracing::{
    span::{Attributes, Id},
    Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

lazy_static! {
    static ref BUSY_HISTOGRAMS: EnumMap<FutureTimingMetric, Histogram> = enum_map! {
        FutureTimingMetric::BuildingVsocksConnection => histogram!(format!("{}_busy", <&str>::from(FutureTimingMetric::BuildingVsocksConnection))),
        FutureTimingMetric::SendingTeeRequest => histogram!(format!("{}_busy", <&str>::from(FutureTimingMetric::SendingTeeRequest))),
        FutureTimingMetric::Health => histogram!(format!("{}_busy", <&str>::from(FutureTimingMetric::Health))),
        FutureTimingMetric::GenerateProof => histogram!(format!("{}_busy", <&str>::from(FutureTimingMetric::GenerateProof))),
        FutureTimingMetric::TeePublicKey => histogram!(format!("{}_busy", <&str>::from(FutureTimingMetric::TeePublicKey))),
    };
    
    static ref IDLE_HISTOGRAMS: EnumMap<FutureTimingMetric, Histogram> = enum_map! {
        FutureTimingMetric::BuildingVsocksConnection => histogram!(format!("{}_idle", <&str>::from(FutureTimingMetric::BuildingVsocksConnection))),
        FutureTimingMetric::SendingTeeRequest => histogram!(format!("{}_idle", <&str>::from(FutureTimingMetric::SendingTeeRequest))),
        FutureTimingMetric::Health => histogram!(format!("{}_idle", <&str>::from(FutureTimingMetric::Health))),
        FutureTimingMetric::GenerateProof => histogram!(format!("{}_idle", <&str>::from(FutureTimingMetric::GenerateProof))),
        FutureTimingMetric::TeePublicKey => histogram!(format!("{}_idle", <&str>::from(FutureTimingMetric::TeePublicKey))),
    };
}

#[derive(Debug, Clone, Copy, EnumIter, EnumString, IntoStaticStr, Enum)]
pub enum FutureTimingMetric {
    #[strum(serialize = "Building_VSOCK_connection")]
    BuildingVsocksConnection,
    #[strum(serialize = "Sending_TEE_request")]
    SendingTeeRequest,
    #[strum(serialize = "health")]
    Health,
    #[strum(serialize = "generate_proof")]
    GenerateProof,
    #[strum(serialize = "tee_public_key")]
    TeePublicKey,
}

impl FutureTimingMetric {
    pub fn by_name(name: &str) -> Option<Self> {
        name.parse().ok()
    }

    pub const fn name(&self) -> &'static str {
        match self {
            FutureTimingMetric::BuildingVsocksConnection => "Building_VSOCK_connection",
            FutureTimingMetric::SendingTeeRequest => "Sending_TEE_request",
            FutureTimingMetric::Health => "health",
            FutureTimingMetric::GenerateProof => "generate_proof",
            FutureTimingMetric::TeePublicKey => "tee_public_key",
        }
    }

    pub fn busy_histogram(&self) -> &'static Histogram {
        &BUSY_HISTOGRAMS[*self]
    }

    pub fn idle_histogram_name(&self) -> &'static Histogram {
        &IDLE_HISTOGRAMS[*self]
    }
}

/// A tracing_subscriber layer that collects timing metrics for spans.
///
/// Based on tracing_subscriber::fmt. Handles spans that are entered and exited
/// multiple times, which is needed to track the time spent in busy and idle states
/// for asynchronous operations.
///
/// The metrics are submitted to the metrics crate as histograms.
#[derive(Debug, Clone)]
pub struct FutureHistogramLayer;

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
        if FutureTimingMetric::iter().any(|metric| metadata.name() == metric.name()) {
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

            let metric =
                FutureTimingMetric::by_name(span.metadata().name()).expect("Invalid metric name");
            metric.busy_histogram().record(micros_to_secs(busy));
            metric.idle_histogram_name().record(micros_to_secs(idle));
        }
    }
}

fn micros_to_secs(micros: u64) -> f64 {
    micros as f64 / 1_000_000.0
}
