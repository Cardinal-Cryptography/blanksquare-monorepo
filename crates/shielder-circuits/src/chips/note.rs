use core::array;

use halo2_proofs::{arithmetic::Field, plonk::Error};
use strum_macros::{EnumCount, EnumIter};

use crate::{
    chips::sum::SumChip,
    consts::POSEIDON_RATE,
    embed::Embed,
    instance_wrapper::InstanceWrapper,
    poseidon::circuit::{hash, PoseidonChip},
    synthesizer::Synthesizer,
    version::NoteVersion,
    AssignedCell, Fr, Value,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, EnumIter, EnumCount)]
pub enum NoteInstance {
    TokenAddress,
}

#[derive(Copy, Clone, Debug)]
pub struct Note<T> {
    pub version: NoteVersion,
    pub id: T,
    pub nullifier: T,
    pub account_balance: T,
    pub token_address: T,
}

impl Embed for Note<Value> {
    type Embedded = Note<AssignedCell>;

    fn embed(
        &self,
        synthesizer: &mut impl Synthesizer,
        annotation: impl Into<alloc::string::String>,
    ) -> Result<Self::Embedded, Error> {
        let annotation = annotation.into();

        Ok(Note {
            version: self.version,
            id: self.id.embed(synthesizer, annotation.clone())?,
            nullifier: self.nullifier.embed(synthesizer, annotation.clone())?,
            account_balance: self
                .account_balance
                .embed(synthesizer, annotation.clone())?,
            token_address: self.token_address.embed(synthesizer, annotation)?,
        })
    }
}

pub mod off_circuit {
    use halo2_proofs::arithmetic::Field;

    use crate::{chips::note::Note, consts::POSEIDON_RATE, poseidon::off_circuit::hash, Fr};

    pub fn note_hash(note: &Note<Fr>) -> Fr {
        let balance_hash = hash::<POSEIDON_RATE>(&[
            note.account_balance,
            note.token_address,
            Fr::ZERO,
            Fr::ZERO,
            Fr::ZERO,
            Fr::ZERO,
            Fr::ZERO,
        ]);

        let input = [
            note.version.as_field(),
            note.id,
            note.nullifier,
            balance_hash,
        ];

        hash(&input)
    }
}

/// Chip that is able to calculate note hash
#[derive(Clone, Debug)]
pub struct NoteChip {
    pub public_inputs: InstanceWrapper<NoteInstance>,

    pub sum: SumChip,
    pub poseidon: PoseidonChip,
}

impl NoteChip {
    fn assign_note_version(
        &self,
        note: &Note<AssignedCell>,
        synthesizer: &mut impl Synthesizer,
    ) -> Result<AssignedCell, Error> {
        let note_version: Fr = note.version.as_field();
        synthesizer.assign_constant("note_version", note_version)
    }

    /// Calculates the note_hash as follows:
    ///
    ///   `note_hash = poseidon2(NOTE_VERSION, note.id, note.nullifier,
    ///                          poseidon2(note.balance, note.token_address, 0, 0, 0, 0, 0))`
    ///
    /// The reason for the double nesting and for the padding is historical: we keep this hash shape
    /// for backward compatibility with notes created by the 1st version of Shielder.
    ///
    /// Constrains `note.token_address` to match the respective public input.
    pub fn note_hash(
        &self,
        synthesizer: &mut impl Synthesizer,
        note: &Note<AssignedCell>,
    ) -> Result<AssignedCell, Error> {
        let note_version = self.assign_note_version(note, synthesizer)?;

        let h_balance = self.balance_hash(synthesizer, note)?;

        self.public_inputs.constrain_cells(
            synthesizer,
            [(note.token_address.clone(), NoteInstance::TokenAddress)],
        )?;

        let input = [
            note_version,
            note.id.clone(),
            note.nullifier.clone(),
            h_balance,
        ];

        hash(synthesizer, self.poseidon.clone(), input)
    }

    fn balance_hash(
        &self,
        synthesizer: &mut impl Synthesizer,
        note: &Note<AssignedCell>,
    ) -> Result<AssignedCell, Error> {
        let zero_cell = synthesizer.assign_constant("Zero", Fr::ZERO)?;

        let mut input: [_; POSEIDON_RATE] = array::from_fn(|_| zero_cell.clone());
        input[0] = note.account_balance.clone();
        input[1] = note.token_address.clone();

        hash(synthesizer, self.poseidon.clone(), input)
    }

    pub fn increase_balance(
        &self,
        synthesizer: &mut impl Synthesizer,
        balance_old: AssignedCell,
        increase_value: AssignedCell,
    ) -> Result<AssignedCell, Error> {
        let balance_new = synthesizer
            .assign_value("balance_new", balance_old.value() + increase_value.value())?;

        self.sum.constrain_sum(
            synthesizer,
            balance_old,
            increase_value,
            balance_new.clone(),
        )?;

        Ok(balance_new)
    }

    pub fn decrease_balance(
        &self,
        synthesizer: &mut impl Synthesizer,
        balance_old: AssignedCell,
        decrease_value: AssignedCell,
    ) -> Result<AssignedCell, Error> {
        let balance_new = synthesizer
            .assign_value("balance_new", balance_old.value() - decrease_value.value())?;

        self.sum.constrain_sum(
            synthesizer,
            balance_new.clone(),
            decrease_value,
            balance_old,
        )?;

        Ok(balance_new)
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{
        arithmetic::Field,
        circuit::{floor_planner, Layouter},
        plonk::{Advice, Circuit, ConstraintSystem, Error},
    };
    use parameterized::parameterized;
    use strum_macros::{EnumCount, EnumIter};

    use super::{Note, NoteChip, NoteInstance};
    use crate::{
        circuits::test_utils::expect_prover_success_and_run_verification,
        column_pool::{ColumnPool, PreSynthesisPhase},
        config_builder::ConfigsBuilder,
        embed::Embed,
        instance_wrapper::InstanceWrapper,
        poseidon::off_circuit::hash,
        synthesizer::create_synthesizer,
        test_utils::expect_instance_permutation_failures,
        Fr, NoteVersion, Value,
    };

    // Tests `NoteChip`. Constrains the last public input to the output of the function under test.
    #[derive(Clone, Debug)]
    enum TestCircuit {
        TestNoteHash(Note<Value>),
        TestBalanceIncrease((Value, Value)),
        TestBalanceDecrease((Value, Value)),
    }

    impl TestCircuit {
        pub fn note_hash_test(note: Note<impl Into<Fr>>) -> Self {
            TestCircuit::TestNoteHash(Note {
                version: note.version,
                id: Value::known(note.id.into()),
                nullifier: Value::known(note.nullifier.into()),
                account_balance: Value::known(note.account_balance.into()),
                token_address: Value::known(note.token_address.into()),
            })
        }

        pub fn balance_increase_test(
            balance_old: impl Into<Fr>,
            increase_value: impl Into<Fr>,
        ) -> Self {
            TestCircuit::TestBalanceIncrease((
                Value::known(balance_old.into()),
                Value::known(increase_value.into()),
            ))
        }

        pub fn balance_decrease_test(
            balance_old: impl Into<Fr>,
            decrease_value: impl Into<Fr>,
        ) -> Self {
            TestCircuit::TestBalanceDecrease((
                Value::known(balance_old.into()),
                Value::known(decrease_value.into()),
            ))
        }
    }

    #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, EnumIter, EnumCount)]
    pub enum TestInstance {
        TokenAddress,
        ChipOutput,
    }

    impl TryFrom<TestInstance> for NoteInstance {
        type Error = ();

        fn try_from(value: TestInstance) -> Result<Self, Self::Error> {
            match value {
                TestInstance::TokenAddress => Ok(NoteInstance::TokenAddress),
                _ => Err(()),
            }
        }
    }

    impl Circuit<Fr> for TestCircuit {
        type Config = (
            NoteChip,
            ColumnPool<Advice, PreSynthesisPhase>,
            InstanceWrapper<TestInstance>,
        );
        type FloorPlanner = floor_planner::V1;

        fn without_witnesses(&self) -> Self {
            match self {
                TestCircuit::TestNoteHash(_) => TestCircuit::TestNoteHash(Note {
                    version: NoteVersion::new(0),
                    id: Value::unknown(),
                    nullifier: Value::unknown(),
                    account_balance: Value::unknown(),
                    token_address: Value::unknown(),
                }),
                TestCircuit::TestBalanceIncrease(_) => {
                    TestCircuit::TestBalanceIncrease((Value::unknown(), Value::unknown()))
                }
                TestCircuit::TestBalanceDecrease(_) => {
                    TestCircuit::TestBalanceDecrease((Value::unknown(), Value::unknown()))
                }
            }
        }

        fn configure(meta: &mut ConstraintSystem<Fr>) -> Self::Config {
            let public_inputs = InstanceWrapper::<TestInstance>::new(meta);

            let configs_builder = ConfigsBuilder::new(meta).with_note(public_inputs.narrow());
            let note = configs_builder.note_chip();

            (note, configs_builder.finish(), public_inputs)
        }

        fn synthesize(
            &self,
            (chip, advice_pool, public_inputs): Self::Config,
            mut layouter: impl Layouter<Fr>,
        ) -> Result<(), Error> {
            let advice_pool = advice_pool.start_synthesis();
            let mut synthesizer = create_synthesizer(&mut layouter, &advice_pool);

            let chip_output = match self {
                TestCircuit::TestNoteHash(note) => {
                    let note = note.embed(&mut synthesizer, "note")?;

                    chip.note_hash(&mut synthesizer, &note)?
                }

                TestCircuit::TestBalanceIncrease((balance_old, increase_value)) => {
                    let balance_old = balance_old.embed(&mut synthesizer, "balance_old")?;
                    let increase_value =
                        increase_value.embed(&mut synthesizer, "increase_value")?;

                    chip.increase_balance(&mut synthesizer, balance_old, increase_value)?
                }

                TestCircuit::TestBalanceDecrease((balance_old, decrease_value)) => {
                    let balance_old = balance_old.embed(&mut synthesizer, "balance_old")?;
                    let decrease_value =
                        decrease_value.embed(&mut synthesizer, "decrease_value")?;

                    chip.decrease_balance(&mut synthesizer, balance_old, decrease_value)?
                }
            };

            public_inputs
                .constrain_cells(&mut synthesizer, [(chip_output, TestInstance::ChipOutput)])
        }
    }

    #[parameterized(token_address = {
        Fr::ZERO,
        Fr::ONE,
        Fr::from(2).pow([160]).sub(&Fr::ONE) // Max token address.
    })]
    fn note_hash_is_calculated_correctly(token_address: Fr) {
        let circuit = TestCircuit::note_hash_test(Note {
            version: NoteVersion::new(0),
            id: Fr::from(1),
            nullifier: Fr::from(2),
            account_balance: Fr::from(3),
            token_address,
        });
        let expected_output = hash(&[
            Fr::from(0),
            Fr::from(1),
            Fr::from(2),
            hash(&[
                Fr::from(3),
                token_address,
                Fr::ZERO,
                Fr::ZERO,
                Fr::ZERO,
                Fr::ZERO,
                Fr::ZERO,
            ]),
        ]);
        let pub_input = [token_address, expected_output];

        assert!(expect_prover_success_and_run_verification(circuit, &pub_input).is_ok());
    }

    #[test]
    fn note_hash_output_is_constrained() {
        let circuit = TestCircuit::note_hash_test(Note {
            version: NoteVersion::new(0),
            id: Fr::from(1),
            nullifier: Fr::from(2),
            account_balance: Fr::from(3),
            token_address: Fr::from(4),
        });
        let pub_input = [Fr::from(4), Fr::from(999999)];

        let failures = expect_prover_success_and_run_verification(circuit, &pub_input)
            .expect_err("Verification must fail");

        expect_instance_permutation_failures(
            &failures,
            "permute state", // Region defined in `poseidon-gadget`.
            1,
        );
    }

    #[test]
    fn note_hash_input_is_constrained() {
        let note = Note {
            version: NoteVersion::new(0),
            id: Fr::from(1),
            nullifier: Fr::from(2),
            account_balance: Fr::from(3),
            token_address: Fr::from(4),
        };
        let circuit = TestCircuit::note_hash_test(note);
        let pub_input = [Fr::from(999999), super::off_circuit::note_hash(&note)];

        let failures = expect_prover_success_and_run_verification(circuit, &pub_input)
            .expect_err("Verification must fail");

        expect_instance_permutation_failures(&failures, "note", 0);
    }

    #[parameterized(
        circuit = {
            TestCircuit::balance_increase_test(20, 5),
            TestCircuit::balance_decrease_test(20, 5)
        },
        expected_output = { 25, 15 }
    )]
    fn balance_update_passes(circuit: TestCircuit, expected_output: u64) {
        let token_address = 999; // Irrelevant.
        let pub_input = [token_address, expected_output];

        assert!(
            expect_prover_success_and_run_verification(circuit, &pub_input.map(Fr::from)).is_ok()
        );
    }

    #[parameterized(
        circuit = {
            TestCircuit::balance_increase_test(20, 5),
            TestCircuit::balance_decrease_test(20, 5)
        },
        expected_output = { 26, 16 }
    )]
    fn balance_update_is_constrained(circuit: TestCircuit, expected_output: u64) {
        let token_address = 999; // Irrelevant.
        let pub_input = [token_address, expected_output];

        let failures =
            expect_prover_success_and_run_verification(circuit, &pub_input.map(Fr::from))
                .expect_err("Verification must fail");

        expect_instance_permutation_failures(&failures, "balance_new", 1);
    }
}
