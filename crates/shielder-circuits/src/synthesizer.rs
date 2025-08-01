use alloc::string::String;

use halo2_proofs::{
    circuit::{Cell, Layouter, NamespacedLayouter, Region, Table},
    plonk::{Advice, Challenge, Column, Error, Instance},
};

use crate::{
    column_pool::{AccessColumn, ColumnPool, SynthesisPhase},
    AssignedCell, Fr, Value,
};

/// A `Synthesizer` is a layouter that can also access advice columns with some inner load balancing.
///
/// Under this trait there are 2 methods for namespacing:
/// - `namespace` from `Layouter` trait: it will create `NamespacedLayouter` within a new namespace.
///   This, however, won't implement `Synthesizer` trait (it will lack `ColumnPool` access).
/// - `namespaced` from `Synthesizer` trait: it will create a new `Synthesizer` within a new
///   namespace. This will implement `Synthesizer` trait and is recommended.
pub trait Synthesizer: Layouter<Fr> + AccessColumn<Advice> {
    /// Creates a new namespace for the synthesizer. Analogous to `Layouter::namespace`.
    fn namespaced(&mut self, name: impl Into<String>) -> impl Synthesizer;

    /// Assign single value to a cell in a dedicated region.
    fn assign_value(
        &mut self,
        name: impl Into<String>,
        value: Value,
    ) -> Result<AssignedCell, Error>;

    /// Assign a constant to a cell in a dedicated region.
    fn assign_constant(
        &mut self,
        name: impl Into<String>,
        constant: Fr,
    ) -> Result<AssignedCell, Error>;
}

/// Creates a new synthesizer from a layouter and an advice pool.
pub fn create_synthesizer<'a, L: Layouter<Fr>>(
    layouter: &'a mut L,
    advice_pool: &'a ColumnPool<Advice, SynthesisPhase>,
) -> impl Synthesizer + 'a {
    SynthesizerImpl {
        layouter: layouter.namespace(|| "synthesizer"),
        advice_pool,
    }
}

struct SynthesizerImpl<'a, L: Layouter<Fr>> {
    layouter: NamespacedLayouter<'a, Fr, L>,
    advice_pool: &'a ColumnPool<Advice, SynthesisPhase>,
}

impl<L: Layouter<Fr>> Synthesizer for SynthesizerImpl<'_, L> {
    fn namespaced(&mut self, name: impl Into<String>) -> impl Synthesizer {
        SynthesizerImpl {
            layouter: self.layouter.namespace(|| name),
            advice_pool: self.advice_pool,
        }
    }

    fn assign_value(
        &mut self,
        name: impl Into<String>,
        value: Value,
    ) -> Result<AssignedCell, Error> {
        let name = &name.into();
        let advice = self.get_any_column();
        self.assign_region(
            || name,
            |mut region| region.assign_advice(|| name, advice, 0, || value),
        )
    }

    fn assign_constant(
        &mut self,
        name: impl Into<String>,
        constant: Fr,
    ) -> Result<AssignedCell, Error> {
        let name = name.into();
        let advice = self.get_any_column();
        self.assign_region(
            || name.clone(),
            |mut region| region.assign_advice_from_constant(|| name.clone(), advice, 0, constant),
        )
    }
}

/// Delegate `Layouter` implementation to the inner layouter.
impl<L: Layouter<Fr>> Layouter<Fr> for SynthesizerImpl<'_, L> {
    type Root = L::Root;

    fn assign_region<A, AR, N, NR>(&mut self, name: N, assignment: A) -> Result<AR, Error>
    where
        A: FnMut(Region<'_, Fr>) -> Result<AR, Error>,
        N: Fn() -> NR,
        NR: Into<String>,
    {
        self.layouter.assign_region(name, assignment)
    }

    fn assign_table<A, N, NR>(&mut self, name: N, assignment: A) -> Result<(), Error>
    where
        A: FnMut(Table<'_, Fr>) -> Result<(), Error>,
        N: Fn() -> NR,
        NR: Into<String>,
    {
        self.layouter.assign_table(name, assignment)
    }

    fn constrain_instance(
        &mut self,
        cell: Cell,
        column: Column<Instance>,
        row: usize,
    ) -> Result<(), Error> {
        self.layouter.constrain_instance(cell, column, row)
    }

    fn get_challenge(&self, challenge: Challenge) -> Value {
        self.layouter.get_challenge(challenge)
    }

    fn get_root(&mut self) -> &mut Self::Root {
        self.layouter.get_root()
    }

    fn push_namespace<NR: Into<String>, N: FnOnce() -> NR>(&mut self, name_fn: N) {
        self.layouter.push_namespace(name_fn)
    }

    fn pop_namespace(&mut self, gadget_name: Option<String>) {
        self.layouter.pop_namespace(gadget_name)
    }
}

/// Delegate `AccessColumn` implementation to the inner advice pool.
impl<L: Layouter<Fr>> AccessColumn<Advice> for SynthesizerImpl<'_, L> {
    fn get_any_column(&self) -> Column<Advice> {
        self.advice_pool.get_any_column()
    }

    fn get_column(&self, index: usize) -> Column<Advice> {
        self.advice_pool.get_column(index)
    }

    fn get_column_array<const N: usize>(&self) -> [Column<Advice>; N] {
        self.advice_pool.get_column_array()
    }
}
