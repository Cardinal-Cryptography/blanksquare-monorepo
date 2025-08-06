use lazy_static::lazy_static;
use metrics::{histogram, Histogram};
use strum::{EnumIter, IntoEnumIterator as _};
use tokio::time::Instant;
use tracing::{
    span::{Attributes, Id},
    Subscriber,
};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

const BUILDING_VSOCKS_CONNECTION: &str = "Building_VSOCK_connection";
lazy_static! {
    static ref BUILDING_VSOCKS_CONNECTION_BUSY: Histogram =
        histogram!(format!("{}_busy", BUILDING_VSOCKS_CONNECTION));
    static ref BUILDING_VSOCKS_CONNECTION_IDLE: Histogram =
        histogram!(format!("{}_idle", BUILDING_VSOCKS_CONNECTION));
}

const SENDING_TEE_REQUEST: &str = "Sending_TEE_request";
lazy_static! {
    static ref SENDING_TEE_REQUEST_BUSY: Histogram =
        histogram!(format!("{}_busy", SENDING_TEE_REQUEST));
    static ref SENDING_TEE_REQUEST_IDLE: Histogram =
        histogram!(format!("{}_idle", SENDING_TEE_REQUEST));
}

const HEALTH: &str = "health";
lazy_static! {
    static ref HEALTH_BUSY: Histogram = histogram!(format!("{}_busy", HEALTH));
    static ref HEALTH_IDLE: Histogram = histogram!(format!("{}_idle", HEALTH));
}

const GENERATE_PROOF: &str = "generate_proof";
lazy_static! {
    static ref GENERATE_PROOF_BUSY: Histogram = histogram!(format!("{}_busy", GENERATE_PROOF));
    static ref GENERATE_PROOF_IDLE: Histogram = histogram!(format!("{}_idle", GENERATE_PROOF));
}

const TEE_PUBLIC_KEY: &str = "tee_public_key";
lazy_static! {
    static ref TEE_PUBLIC_KEY_BUSY: Histogram = histogram!(format!("{}_busy", TEE_PUBLIC_KEY));
    static ref TEE_PUBLIC_KEY_IDLE: Histogram = histogram!(format!("{}_idle", TEE_PUBLIC_KEY));
}

#[derive(Debug, Clone, Copy, EnumIter)]
pub enum FutureTimingMetric {
    BuildingVsocksConnection,
    SendingTeeRequest,
    Health,
    GenerateProof,
    TeePublicKey,
}

impl FutureTimingMetric {
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            BUILDING_VSOCKS_CONNECTION => Some(FutureTimingMetric::BuildingVsocksConnection),
            SENDING_TEE_REQUEST => Some(FutureTimingMetric::SendingTeeRequest),
            HEALTH => Some(FutureTimingMetric::Health),
            GENERATE_PROOF => Some(FutureTimingMetric::GenerateProof),
            TEE_PUBLIC_KEY => Some(FutureTimingMetric::TeePublicKey),
            _ => None,
        }
    }

    pub const fn name(&self) -> &'static str {
        match self {
            FutureTimingMetric::BuildingVsocksConnection => BUILDING_VSOCKS_CONNECTION,
            FutureTimingMetric::SendingTeeRequest => SENDING_TEE_REQUEST,
            FutureTimingMetric::Health => HEALTH,
            FutureTimingMetric::GenerateProof => GENERATE_PROOF,
            FutureTimingMetric::TeePublicKey => TEE_PUBLIC_KEY,
        }
    }

    pub fn busy_histogram(&self) -> &'static Histogram {
        match self {
            FutureTimingMetric::BuildingVsocksConnection => &BUILDING_VSOCKS_CONNECTION_BUSY,
            FutureTimingMetric::SendingTeeRequest => &SENDING_TEE_REQUEST_BUSY,
            FutureTimingMetric::Health => &HEALTH_BUSY,
            FutureTimingMetric::GenerateProof => &GENERATE_PROOF_BUSY,
            FutureTimingMetric::TeePublicKey => &TEE_PUBLIC_KEY_BUSY,
        }
    }

    pub fn idle_histogram_name(&self) -> &'static Histogram {
        match self {
            FutureTimingMetric::BuildingVsocksConnection => &BUILDING_VSOCKS_CONNECTION_IDLE,
            FutureTimingMetric::SendingTeeRequest => &SENDING_TEE_REQUEST_IDLE,
            FutureTimingMetric::Health => &HEALTH_IDLE,
            FutureTimingMetric::GenerateProof => &GENERATE_PROOF_IDLE,
            FutureTimingMetric::TeePublicKey => &TEE_PUBLIC_KEY_IDLE,
        }
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
