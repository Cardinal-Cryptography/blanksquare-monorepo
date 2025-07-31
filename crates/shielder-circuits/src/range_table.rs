use alloc::format;

use halo2_proofs::{
    circuit::{Table, Value},
    halo2curves::ff::PrimeField,
    plonk::{ConstraintSystem, Error, TableColumn, TableError},
};

use crate::{synthesizer::Synthesizer, Fr};

/// Represents a set of field elements between `0` and `2^RANGE_LOG`. Can be used for lookups.
#[derive(Clone, Debug)]
pub struct RangeTable<const RANGE_LOG: usize> {
    column: TableColumn,
}

impl<const RANGE_LOG: usize> RangeTable<RANGE_LOG> {
    /// Creates a new range table by creating a new dedicated lookup table column.
    pub fn new(cs: &mut ConstraintSystem<Fr>) -> Self {
        assert!(
            RANGE_LOG <= Fr::CAPACITY as usize,
            "RANGE_LOG is too large for the field"
        );
        Self {
            column: cs.lookup_table_column(),
        }
    }

    /// Returns the column of the range table.
    pub fn column(&self) -> TableColumn {
        self.column
    }

    /// Initializes the range table if it has not been initialized yet. Otherwise, does nothing.
    pub fn ensure_initialized(&self, synthesizer: &mut impl Synthesizer) -> Result<(), Error> {
        synthesizer.assign_table(
            || "Range table",
            |mut table| {
                // Check if the table has already been initialized.
                if self.check_initialization(&mut table)? {
                    return Ok(());
                }

                // If not, initialize the rest of the table.
                for index in 1..(1 << RANGE_LOG) {
                    Self::initialize_cell(&mut table, self.column, index)?;
                }
                Ok(())
            },
        )
    }

    /// Checks if the table has already been initialized by trying initializing the first cell.
    fn check_initialization(&self, table: &mut Table<Fr>) -> Result<bool, Error> {
        match Self::initialize_cell(table, self.column, 0) {
            Ok(_) => Ok(false), // Not yet initialized
            Err(Error::TableError(TableError::UsedColumn(_))) => Ok(true), // Already initialized
            Err(e) => Err(e),   // Propagate other errors
        }
    }

    fn initialize_cell(
        table: &mut Table<Fr>,
        column: TableColumn,
        index: usize,
    ) -> Result<(), Error> {
        table.assign_cell(
            || format!("cell with {index}"),
            column,
            index,
            || Value::known(Fr::from(index as u64)),
        )
    }
}
