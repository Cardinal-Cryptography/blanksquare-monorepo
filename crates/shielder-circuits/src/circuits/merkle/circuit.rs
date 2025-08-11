use halo2_proofs::{
    circuit::{floor_planner::V1, Layouter},
    plonk::{Advice, Circuit, ConstraintSystem, Error},
};

use crate::{
    circuits::merkle::knowledge::MerkleProverKnowledge,
    column_pool::{ColumnPool, PreSynthesisPhase},
    config_builder::ConfigsBuilder,
    embed::Embed,
    instance_wrapper::InstanceWrapper,
    merkle::{chip::MerkleChip, MerkleInstance},
    synthesizer::create_synthesizer,
    Fr, Value,
};

#[derive(Clone, Debug, Default)]
pub struct MerkleCircuit<const TREE_HEIGHT: usize>(pub MerkleProverKnowledge<TREE_HEIGHT, Value>);

impl<const TREE_HEIGHT: usize> Circuit<Fr> for MerkleCircuit<TREE_HEIGHT> {
    type Config = (MerkleChip, ColumnPool<Advice, PreSynthesisPhase>);
    type FloorPlanner = V1;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
        let public_inputs = InstanceWrapper::<MerkleInstance>::new(meta);
        let configs_builder = ConfigsBuilder::new(meta).with_merkle(public_inputs);
        (configs_builder.merkle_chip(), configs_builder.finish())
    }

    fn synthesize(
        &self,
        (main_chip, column_pool): Self::Config,
        mut layouter: impl Layouter<Fr>,
    ) -> Result<(), Error> {
        let pool = column_pool.start_synthesis();
        let mut synthesizer = create_synthesizer(&mut layouter, &pool);
        let knowledge = self.0.embed(&mut synthesizer, "MerkleProverKnowledge")?;
        main_chip.synthesize(&mut synthesizer, &knowledge)
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::halo2curves::bn256::Fr;

    use crate::{
        circuits::{merkle::knowledge::MerkleProverKnowledge, test_utils::run_full_pipeline},
        consts::merkle_constants::NOTE_TREE_HEIGHT,
    };

    #[test]
    fn positive_pipeline_for_merkle_proof_circuit() {
        run_full_pipeline::<MerkleProverKnowledge<{ NOTE_TREE_HEIGHT }, Fr>>()
    }
}
