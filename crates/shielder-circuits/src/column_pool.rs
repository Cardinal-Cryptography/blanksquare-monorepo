use alloc::vec::Vec;
use core::{cell::RefCell, marker::PhantomData};

use halo2_proofs::plonk::{Advice, Column, ColumnType, ConstraintSystem, Fixed};

use crate::Fr;

pub enum ConfigPhase {}
/// This is kind of an artificial intermediate phase. Since we do some load balancing in the pool,
/// `ColumnPool` is stateful, and thus we want to prevent cloning it during any phase. However,
/// the pool will usually be a part of `Circuit::Config` type, which is required to be cloneable
/// (for example, `Circuit::synthesize` might be called multiple times). Therefore, we allow cloning
/// the pool only after the configuration phase is concluded and before the synthesis phase starts.
pub enum PreSynthesisPhase {}
pub enum SynthesisPhase {}

/// Column management for the constraint system.
///
/// Depending on the circuit building phase, different operations are allowed:
/// - In the configuration phase, columns can be added to the pool and accessed. Pool cannot be
///   cloned.
/// - In the pre-synthesis phase, pool can be cloned, but it is not possible to add nor access
///   columns.
/// - In the synthesis phase, pool cannot be cloned. Columns can be accessed, but not added.
#[derive(Debug)]
pub struct ColumnPool<C: ColumnType, Phase> {
    pool: Vec<Column<C>>,
    access_counter: RefCell<Vec<usize>>,
    _phantom: PhantomData<Phase>,
}

/// Pool can be cloned only during pre-synthesis phase.
impl<C: ColumnType> Clone for ColumnPool<C, PreSynthesisPhase> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            access_counter: self.access_counter.clone(),
            _phantom: Default::default(),
        }
    }
}

impl<C: ColumnType> ColumnPool<C, ConfigPhase> {
    /// Create a new empty pool for the configuration phase.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            pool: Default::default(),
            access_counter: Default::default(),
            _phantom: Default::default(),
        }
    }
}

// ===================== PHASE TRANSITIONS ====================================================== //

impl<C: ColumnType> ColumnPool<C, ConfigPhase> {
    /// Finish the configuration phase and move to the pre-synthesis phase.
    pub fn conclude_configuration(self) -> ColumnPool<C, PreSynthesisPhase> {
        ColumnPool {
            pool: self.pool,
            access_counter: self.access_counter,
            _phantom: Default::default(),
        }
    }
}

impl<C: ColumnType> ColumnPool<C, PreSynthesisPhase> {
    /// Move to the synthesis phase.
    pub fn start_synthesis(self) -> ColumnPool<C, SynthesisPhase> {
        ColumnPool {
            pool: self.pool,
            access_counter: self.access_counter,
            _phantom: Default::default(),
        }
    }
}

// ===================== CREATING COLUMNS ======================================================= //

impl<C: ColumnType> ColumnPool<C, ConfigPhase> {
    fn add_column(&mut self, column: Column<C>) {
        self.pool.push(column);
        self.access_counter.borrow_mut().push(0);
    }
}

impl ColumnPool<Advice, ConfigPhase> {
    /// Ensure that there are at least `capacity` advice columns in the constraint system `cs`,
    /// registering new ones if necessary. Enable equality for each of them.
    pub fn ensure_capacity(&mut self, cs: &mut ConstraintSystem<Fr>, capacity: usize) {
        for _ in self.pool.len()..capacity {
            let column = cs.advice_column();
            cs.enable_equality(column);
            self.add_column(column);
        }
    }
}

impl ColumnPool<Fixed, ConfigPhase> {
    /// Ensure that there are at least `capacity` fixed columns in the constraint system `cs`,
    /// registering new ones if necessary. Enable storing constants in each of them.
    pub fn ensure_capacity(&mut self, cs: &mut ConstraintSystem<Fr>, capacity: usize) {
        for _ in self.pool.len()..capacity {
            let column = cs.fixed_column();
            cs.enable_constant(column);
            self.add_column(column);
        }
    }
}

// ===================== ACCESSING COLUMNS ====================================================== //

trait PhaseWithAccess {}
impl PhaseWithAccess for ConfigPhase {}
impl PhaseWithAccess for SynthesisPhase {}

pub trait AccessColumn<C: ColumnType> {
    /// Get some advice column from the pool.
    ///
    /// The index is not guaranteed (some inner load balancing might be applied).
    fn get_any_column(&self) -> Column<C>;

    /// Get the column at the specified index.
    fn get_column(&self, index: usize) -> Column<C>;

    /// Get an array of columns from the pool.
    fn get_column_array<const N: usize>(&self) -> [Column<C>; N];
}

#[allow(private_bounds)]
impl<C: ColumnType, Phase: PhaseWithAccess> AccessColumn<C> for ColumnPool<C, Phase> {
    fn get_any_column(&self) -> Column<C> {
        let idx = self
            .access_counter
            .borrow()
            .iter()
            .enumerate()
            .min_by_key(|&(_, count)| count)
            .map(|(idx, _)| idx)
            .expect("empty pool");
        self.get_column(idx)
    }

    fn get_column(&self, index: usize) -> Column<C> {
        self.access_counter.borrow_mut()[index] += 1;
        self.pool[index]
    }

    fn get_column_array<const N: usize>(&self) -> [Column<C>; N] {
        for i in 0..N {
            self.access_counter.borrow_mut()[i] += 1;
        }
        self.pool[..N].try_into().unwrap()
    }
}
