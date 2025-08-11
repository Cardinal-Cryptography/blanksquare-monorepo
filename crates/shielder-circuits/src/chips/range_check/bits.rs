use alloc::vec::Vec;

use halo2_proofs::halo2curves::ff::PrimeFieldBits;

use crate::{Fr, Value};

/// Splits least significant bits of a field value `value` into `chunks` chunks of size `chunk_size`
/// each, ensuring any leading bits are zero.
///
/// The chunks returned are in little-endian order. For example, when given a value `0b1001_0110`,
/// `chunk_size = 4`, and `chunks = 2`, the function will return `[0b0110, 0b1001]`. If
/// `chunks = 3`, the function will return `[0b0110, 0b1001, 0b0000]`.
///
/// # Returns
///
/// A `Vec<Value>`, where each `Value` corresponds to one chunk, represented as a finite field
/// element. Vector is guaranteed to have `chunks` elements (padded with 0s if necessary).
///
/// # Panics for the Prover (when `value` is known)
///
/// The function will panic if the input does not contain enough bits (`chunk_size * CHUNKS`).
/// The function will panic if trailing bits are non-zero.
/// The function will panic if `chunk_size` is greater than 64.
pub fn to_chunks(value: Value, chunk_size: usize, chunks: usize) -> Vec<Value> {
    assert!(chunk_size <= 64, "Chunk size must be <= 64");

    // Take LittleEndian bits of the value.
    let bits = value.map(|value| value.to_le_bits().into_iter().collect::<Vec<_>>());

    // Sanity check for the prover, that we have enough bits (possibly trailing bits are 0s).
    bits.assert_if_known(|bits| bits.len() >= chunk_size * chunks);

    // Discard trailing bits. They all must be 0s.
    let (prefix, suffix) = bits
        .map(|bits| {
            let (prefix, suffix) = bits.split_at(chunk_size * chunks);
            (prefix.to_vec(), suffix.to_vec())
        })
        .unzip();

    // Sanity check for the prover, that the trailing bits are all 0s.
    suffix.assert_if_known(|suffix| suffix.iter().all(|bit| !bit));

    // Convert bit chunks back to the field.
    let bit_chunks = prefix.map(|bits| {
        bits.chunks_exact(chunk_size)
            .map(|chunk| Fr::from(bits_to_u64(chunk)))
            .collect::<Vec<_>>()
    });

    // Transpose data.
    bit_chunks.transpose_vec(chunks)
}

/// Converts a little-endian bit slice to an integer (u64).
///
/// # Parameters
///
/// `bits`: A slice of booleans representing bits in little-endian order. The least significant bit
/// (LSB) is at index 0.
///
/// # Returns
///
/// The `u64` integer representation of the bit slice.
///
/// # Panics
///
/// This function will panic if the length of the bit slice exceeds 64.
fn bits_to_u64(bits: &[bool]) -> u64 {
    assert!(bits.len() <= 64, "Bit slice length must be <= 64");
    bits.iter()
        .enumerate()
        .fold(0u64, |acc, (i, &b)| acc | ((b as u64) << i))
}

#[cfg(test)]
mod tests {
    use halo2_proofs::halo2curves::bn256::Fr;

    use super::*;

    mod bits_to_u64 {
        use super::*;

        #[test]
        fn basic() {
            let bits = [true, false, true]; // Binary: 101 (little-endian)
            assert_eq!(bits_to_u64(&bits), 5);
        }

        #[test]
        fn all_false() {
            let bits = [false; 10]; // Binary: 0000000000
            assert_eq!(bits_to_u64(&bits), 0);
        }

        #[test]
        fn all_true() {
            let bits = [true; 5]; // Binary: 11111 (little-endian)
            assert_eq!(bits_to_u64(&bits), 31);
        }

        #[test]
        fn mixed() {
            let bits = [false, true, false, true, true]; // Binary: 01011 (little-endian)
            assert_eq!(bits_to_u64(&bits), 26);
        }

        #[test]
        fn empty() {
            let bits: [bool; 0] = []; // Empty slice
            assert_eq!(bits_to_u64(&bits), 0);
        }

        #[test]
        #[should_panic(expected = "Bit slice length must be <= 64")]
        fn panic() {
            let bits = [false; 65]; // Exceeds 64 bits
            bits_to_u64(&bits); // Should panic
        }
    }

    mod to_chunks {
        use super::*;
        use crate::Field;

        const CHUNK_SIZE: usize = 4;
        const CHUNKS: usize = 2;

        #[test]
        fn basic() {
            let value = Value::known(Fr::from(0b1001_0110u64));

            let result = to_chunks(value, CHUNK_SIZE, CHUNKS);

            assert_eq!(result.len(), CHUNKS);
            result[0].assert_if_known(|v| *v == Fr::from(0b0110u64));
            result[1].assert_if_known(|v| *v == Fr::from(0b1001u64));
        }

        #[test]
        fn short_number() {
            let value = Value::known(Fr::from(0b1001u64));

            let result = to_chunks(value, CHUNK_SIZE, CHUNKS);

            assert_eq!(result.len(), CHUNKS);
            result[0].assert_if_known(|v| *v == Fr::from(0b1001u64));
            result[1].assert_if_known(|v| *v == Fr::ZERO);
        }

        #[test]
        #[should_panic]
        fn nonzero_trailing_bits() {
            let value = Value::known(Fr::from(0b1_0000_0000u64));
            to_chunks(value, CHUNK_SIZE, CHUNKS);
        }

        #[test]
        #[should_panic]
        fn insufficient_bits() {
            let value = Value::known(Fr::ZERO);
            to_chunks(value, 64, 1000); // Should panic, because Fr has less than 64K bits
        }
    }
}
