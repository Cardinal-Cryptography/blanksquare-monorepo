use halo2_proofs::plonk::{Advice, ConstraintSystem, Fixed};

use crate::{
    chips::{
        el_gamal::ElGamalEncryptionChip,
        note::{NoteChip, NoteInstance},
        points_add::PointsAddChip,
        range_check::RangeCheckChip,
        scalar_multiply::ScalarMultiplyChip,
        sum::SumChip,
        to_affine::ToAffineChip,
        to_projective::ToProjectiveChip,
    },
    column_pool::{AccessColumn, ColumnPool, ConfigPhase, PreSynthesisPhase},
    consts::merkle_constants::WIDTH,
    gates::{
        is_point_on_curve_affine::IsPointOnCurveAffineGate, membership::MembershipGate,
        points_add::PointsAddGate, scalar_multiply::ScalarMultiplyGate, sum::SumGate,
        to_affine::ToAffineGate, Gate,
    },
    instance_wrapper::InstanceWrapper,
    merkle::{MerkleChip, MerkleInstance},
    poseidon::{circuit::PoseidonChip, spec::PoseidonSpec},
    Fr,
};

pub struct ConfigsBuilder<'cs> {
    system: &'cs mut ConstraintSystem<Fr>,
    advice_pool: ColumnPool<Advice, ConfigPhase>,
    fixed_pool: ColumnPool<Fixed, ConfigPhase>,

    merkle: Option<MerkleChip>,
    poseidon: Option<PoseidonChip>,
    range_check: Option<RangeCheckChip>,
    sum: Option<SumChip>,
    points_add: Option<PointsAddChip>,
    scalar_multiply: Option<ScalarMultiplyChip>,
    to_affine: Option<ToAffineChip>,
    to_projective: Option<ToProjectiveChip>,
    is_point_on_curve_affine: Option<IsPointOnCurveAffineGate>,
    el_gamal_encryption: Option<ElGamalEncryptionChip>,
    note: Option<NoteChip>,
}

macro_rules! check_if_cached {
    ($self:ident, $field:ident) => {
        if $self.$field.is_some() {
            return $self;
        }
    };
}

impl<'cs> ConfigsBuilder<'cs> {
    pub fn new(system: &'cs mut ConstraintSystem<Fr>) -> Self {
        Self {
            system,
            advice_pool: ColumnPool::<Advice, _>::new(),
            fixed_pool: ColumnPool::<Fixed, _>::new(),

            merkle: None,
            poseidon: None,
            range_check: None,
            sum: None,
            points_add: None,
            scalar_multiply: None,
            to_affine: None,
            to_projective: None,
            is_point_on_curve_affine: None,
            el_gamal_encryption: None,
            note: None,
        }
    }

    pub fn finish(self) -> ColumnPool<Advice, PreSynthesisPhase> {
        self.advice_pool.conclude_configuration()
    }

    pub fn with_poseidon(mut self) -> Self {
        check_if_cached!(self, poseidon);

        let advice_pool = self.advice_pool_with_capacity(WIDTH + 1);
        let advice_array = advice_pool.get_column_array::<WIDTH>();
        let advice = advice_pool.get_column(WIDTH);

        let fixed_pool = self.fixed_pool_with_capacity(WIDTH);
        let fixed_array = fixed_pool.get_column_array::<WIDTH>();

        let poseidon_config =
            PoseidonChip::configure::<PoseidonSpec>(self.system, advice_array, fixed_array, advice);

        self.poseidon = Some(PoseidonChip::construct(poseidon_config));
        self
    }

    pub fn poseidon_chip(&self) -> PoseidonChip {
        self.poseidon.clone().expect("Poseidon not configured")
    }

    pub fn with_merkle(mut self, public_inputs: InstanceWrapper<MerkleInstance>) -> Self {
        check_if_cached!(self, merkle);
        self = self.with_poseidon();

        self.merkle = Some(MerkleChip {
            membership_gate: MembershipGate::create_gate(self.system, &mut self.advice_pool),
            public_inputs,
            poseidon: self.poseidon_chip(),
        });
        self
    }

    pub fn merkle_chip(&self) -> MerkleChip {
        self.merkle.clone().expect("Merkle not configured")
    }

    pub fn with_range_check(mut self) -> Self {
        check_if_cached!(self, range_check);
        self = self.with_sum();

        let system = &mut self.system;
        self.range_check = Some(RangeCheckChip::new(
            system,
            &mut self.advice_pool,
            self.sum.clone().unwrap(),
        ));
        self
    }

    pub fn range_check_chip(&self) -> RangeCheckChip {
        self.range_check.clone().expect("RangeCheck not configured")
    }

    pub fn with_sum(mut self) -> Self {
        check_if_cached!(self, sum);
        self.sum = Some(SumChip::new(SumGate::create_gate(
            self.system,
            &mut self.advice_pool,
        )));
        self
    }

    pub fn sum_chip(&self) -> SumChip {
        self.sum.clone().expect("Sum not configured")
    }

    pub fn with_points_add_chip(mut self) -> Self {
        check_if_cached!(self, points_add);
        self.points_add = Some(PointsAddChip {
            gate: PointsAddGate::create_gate(self.system, &mut self.advice_pool),
        });
        self
    }

    pub fn points_add_chip(&self) -> PointsAddChip {
        self.points_add
            .clone()
            .expect("PointAddChip not configured")
    }

    pub fn with_scalar_multiply_chip(mut self) -> Self {
        check_if_cached!(self, scalar_multiply);
        self.scalar_multiply = Some(ScalarMultiplyChip {
            multiply_gate: ScalarMultiplyGate::create_gate(self.system, &mut self.advice_pool),
        });
        self
    }

    pub fn scalar_multiply_chip(&self) -> ScalarMultiplyChip {
        self.scalar_multiply
            .clone()
            .expect("ScalarMultiplyChip is not configured")
    }

    pub fn with_to_affine_chip(mut self) -> Self {
        check_if_cached!(self, to_affine);
        self.to_affine = Some(ToAffineChip {
            gate: ToAffineGate::create_gate(self.system, &mut self.advice_pool),
        });
        self
    }

    pub fn to_affine_chip(&self) -> ToAffineChip {
        self.to_affine
            .clone()
            .expect("ToAffine chip is not configured")
    }

    pub fn with_to_projective_chip(mut self) -> Self {
        check_if_cached!(self, to_projective);
        self.to_projective = Some(ToProjectiveChip::new());
        self
    }

    pub fn to_projective_chip(&self) -> ToProjectiveChip {
        self.to_projective
            .clone()
            .expect("ToProjective chip is not configured")
    }

    pub fn with_is_point_on_curve_affine(mut self) -> Self {
        check_if_cached!(self, is_point_on_curve_affine);
        self.is_point_on_curve_affine = Some(IsPointOnCurveAffineGate::create_gate(
            self.system,
            &mut self.advice_pool,
        ));
        self
    }

    pub fn is_point_on_curve_affine_gate(&self) -> IsPointOnCurveAffineGate {
        self.is_point_on_curve_affine
            .expect("IsPointOnCurveAffineGate is not configured")
    }

    pub fn with_note(mut self, public_inputs: InstanceWrapper<NoteInstance>) -> Self {
        check_if_cached!(self, note);
        self = self.with_sum();
        self = self.with_poseidon();

        self.note = Some(NoteChip {
            public_inputs,
            sum: self.sum_chip(),
            poseidon: self.poseidon_chip(),
        });
        self
    }

    pub fn note_chip(&self) -> NoteChip {
        self.note.clone().expect("Note not configured")
    }

    pub fn with_el_gamal_encryption_chip(mut self) -> Self {
        check_if_cached!(self, el_gamal_encryption);
        self = self.with_sum();
        self = self.with_points_add_chip();
        self = self.with_scalar_multiply_chip();

        self.el_gamal_encryption = Some(ElGamalEncryptionChip {
            multiply_chip: self.scalar_multiply_chip(),
            add_chip: self.points_add_chip(),
            sum_chip: self.sum_chip(),
        });
        self
    }

    pub fn el_gamal_encryption_chip(&self) -> ElGamalEncryptionChip {
        self.el_gamal_encryption
            .clone()
            .expect("ElGamalEncryptionChip not configured")
    }

    pub fn advice_pool_with_capacity(
        &mut self,
        capacity: usize,
    ) -> &ColumnPool<Advice, ConfigPhase> {
        self.advice_pool.ensure_capacity(self.system, capacity);
        &self.advice_pool
    }

    fn fixed_pool_with_capacity(&mut self, capacity: usize) -> &ColumnPool<Fixed, ConfigPhase> {
        self.fixed_pool.ensure_capacity(self.system, capacity);
        &self.fixed_pool
    }
}
