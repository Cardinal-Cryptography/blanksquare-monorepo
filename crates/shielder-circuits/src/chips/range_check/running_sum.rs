use alloc::{vec, vec::Vec};

use crate::{chips::range_check::bits::to_chunks, Field, Fr, Value};

/// Computes the running sum of a value. The sum will consist of `chunks + 1` values, satisfying:
///  - `z_i = 2^chunk_size * z_{i + 1} + a_i`
///  - `z_0 = value`
///  - `z_{chunks} = 0`
///
/// where `a_i` are the mentioned chunks.
///
/// # Panics for the Prover (when `value` is known)
///
/// The function will panic if the input value is greater or equal than `2^(chunk_size * chunks)`.
/// The function will panic, if `F` has less than `chunk_size * chunks` bits in its representation.
pub fn running_sum(value: Value, chunk_size: usize, chunks: usize) -> Vec<Value> {
    let chunks = to_chunks(value, chunk_size, chunks);

    // Precompute the inverse of `2^CHUNK_SIZE`.
    let inv_two_pow = Fr::from(2).pow([chunk_size as u64]).invert().unwrap();

    let mut current_sum = value;
    let mut running_sum = vec![current_sum];

    for chunk in chunks {
        let next_sum = current_sum
            .zip(chunk)
            .map(|(current_sum, chunk)| (current_sum - chunk) * inv_two_pow);
        current_sum = next_sum;
        running_sum.push(current_sum);
    }

    // Sanity check for the prover, that we have generated correct running sum.
    current_sum.assert_if_known(|v| *v == Fr::ZERO);

    running_sum
}

#[cfg(test)]
mod tests {
    use halo2_proofs::halo2curves::bn256::Fr;

    use super::*;
    use crate::{Field, Value};

    const CHUNK_SIZE: usize = 3;
    const CHUNKS: usize = 3;

    #[test]
    fn zero() {
        let value = Value::known(Fr::from(0u64));

        let result = running_sum(value, CHUNK_SIZE, CHUNKS);

        assert_eq!(result.len(), 4);
        for i in 0..4 {
            result[i].assert_if_known(|v| *v == Fr::ZERO);
        }
    }

    #[test]
    fn basic() {
        let value = Value::known(Fr::from(0b101_010_101u64));

        let result = running_sum(value, CHUNK_SIZE, CHUNKS);

        assert_eq!(result.len(), 4);
        result[0].assert_if_known(|v| *v == Fr::from(0b101_010_101u64)); // Original value
        result[1].assert_if_known(|v| *v == Fr::from(0b101_010u64)); // First two chunks
        result[2].assert_if_known(|v| *v == Fr::from(0b101u64)); // First chunk
        result[3].assert_if_known(|v| *v == Fr::ZERO); // Zero
    }

    #[test]
    #[should_panic]
    fn value_too_large() {
        let value = Value::known(Fr::from((1 << (CHUNK_SIZE * CHUNKS)) + 1));
        running_sum(value, CHUNK_SIZE, CHUNKS);
    }

    #[test]
    #[should_panic]
    fn useless_bound() {
        let value = Value::known(Fr::ZERO);
        running_sum(value, 1000, 1000); // Should panic, because Fr has less than 1M bits
    }
}
