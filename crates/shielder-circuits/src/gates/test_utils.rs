use std::{
    marker::PhantomData,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use halo2_proofs::{
    circuit::{floor_planner::V1, Layouter},
    dev::MockProver,
    plonk::{Advice, Circuit, ConstraintSystem, Error},
};

use crate::{
    column_pool::{ColumnPool, PreSynthesisPhase},
    embed::Embed,
    gates::Gate,
    synthesizer::create_synthesizer,
    Fr,
};

/// The minimal circuit that uses a single gate. It represents a single application of the gate to
/// the input.
///
/// # Configuration phase
///
/// The constraint system will register exactly one gate (after creating required columns).
///
/// # Synthesis phase
///
/// The circuit will perform two steps:
/// 1. Embed the input into the circuit.
/// 2. Apply the gate to the embedded input.
pub struct OneGateCircuit<Gate, Input> {
    input: Input,
    _marker: PhantomData<Gate>,
}

impl<Gate, Input> OneGateCircuit<Gate, Input> {
    /// Create new circuit instance for a single gate application to `input`.
    pub fn new(input: Input) -> Self {
        Self {
            input,
            _marker: PhantomData,
        }
    }
}

impl<G: Gate + Clone, Input: Embed<Embedded = <G as Gate>::Input> + Default> Circuit<Fr>
    for OneGateCircuit<G, Input>
{
    type Config = (ColumnPool<Advice, PreSynthesisPhase>, G);
    type FloorPlanner = V1;

    fn without_witnesses(&self) -> Self {
        Self {
            input: Input::default(),
            _marker: PhantomData,
        }
    }

    /// Our only goal is to register gate `G`. Firstly, we organize sufficient advice area and then
    /// we create the gate instance.
    fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
        let mut advice_pool = ColumnPool::<Advice, _>::new();

        let fixed = meta.fixed_column();
        meta.enable_constant(fixed);

        let gate = G::create_gate(meta, &mut advice_pool);
        (advice_pool.conclude_configuration(), gate)
    }

    /// Embed the input and apply the gate.
    fn synthesize(
        &self,
        (advice_pool, gate): (ColumnPool<Advice, PreSynthesisPhase>, G),
        mut layouter: impl Layouter<Fr>,
    ) -> Result<(), Error> {
        let pool = advice_pool.start_synthesis();
        let mut synthesizer = create_synthesizer(&mut layouter, &pool);
        let embedded_input = self.input.embed(&mut synthesizer, "input")?;
        gate.apply_in_new_region(&mut synthesizer, embedded_input)
    }
}

pub fn verify<G: Gate + Clone, Input: Embed<Embedded = <G as Gate>::Input> + Default>(
    input: Input,
) -> Result<(), Vec<String>> {
    let circuit = OneGateCircuit::<G, Input>::new(input);
    MockProver::run(9, &circuit, vec![])
        .expect("Mock prover should run")
        .verify()
        .map_err(|v| v.into_iter().map(|e| e.to_string()).collect())
}
