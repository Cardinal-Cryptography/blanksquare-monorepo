use alloc::{collections::BTreeMap, format};
use core::{borrow::Borrow, fmt::Debug};

use halo2_proofs::plonk::{Advice, Column, ConstraintSystem, Error, Instance};
use strum::IntoEnumIterator;

use crate::{synthesizer::Synthesizer, AssignedCell, Fr};

#[derive(Clone, Debug)]
pub struct InstanceWrapper<Identifier> {
    column: Column<Instance>,
    offsets: BTreeMap<Identifier, usize>,
}

impl<Identifier: IntoEnumIterator + Ord> InstanceWrapper<Identifier> {
    /// This MUST be called once per circuit. If its components require subset of instance, use
    /// `narrow`.
    pub fn new(meta: &mut ConstraintSystem<Fr>) -> Self {
        let offsets = BTreeMap::from_iter(Identifier::iter().enumerate().map(|(i, id)| (id, i)));

        let column = meta.instance_column();
        meta.enable_equality(column);

        Self { column, offsets }
    }
}

impl<Identifier: IntoEnumIterator + Ord + Debug> InstanceWrapper<Identifier> {
    pub fn copy_as_advice(
        &self,
        synthesizer: &mut impl Synthesizer,
        target_column: Column<Advice>,
        instance: impl Borrow<Identifier>,
    ) -> Result<AssignedCell, Error> {
        let instance = instance.borrow();
        let ann = || format!("{instance:?} as advice");
        let offset = self.offsets[instance];

        synthesizer.assign_region(ann, |mut region| {
            region.assign_advice_from_instance(ann, self.column, offset, target_column, 0)
        })
    }

    /// For every pair `(advice_cell, instance_id)` in `cells`, constrain the `advice_cell` to the
    /// `instance_id`.
    pub fn constrain_cells(
        &self,
        synthesizer: &mut impl Synthesizer,
        cells: impl IntoIterator<Item = (AssignedCell, Identifier)>,
    ) -> Result<(), Error> {
        for (assigned_cell, instance_id) in cells {
            let offset = self.offsets[&instance_id];
            synthesizer.constrain_instance(assigned_cell.cell(), self.column, offset)?;
        }
        Ok(())
    }
}

impl<ParentId: IntoEnumIterator + Ord + Clone> InstanceWrapper<ParentId> {
    /// Create a new instance wrapper with a subset of the instance. It is guaranteed that the
    /// offsets will be compatible with the parent instance.
    pub fn narrow<ChildId: IntoEnumIterator + Ord>(&self) -> InstanceWrapper<ChildId>
    where
        ParentId: TryInto<ChildId>,
    {
        let mut child_offsets = BTreeMap::new();
        for (parent_instance, &parent_offset) in self.offsets.iter() {
            if let Ok(child_instance) = parent_instance.clone().try_into() {
                child_offsets.insert(child_instance, parent_offset);
            }
        }

        assert_eq!(
            child_offsets.len(),
            ChildId::iter().count(),
            "Some child instances are missing"
        );

        InstanceWrapper::<ChildId> {
            column: self.column,
            offsets: child_offsets,
        }
    }
}
