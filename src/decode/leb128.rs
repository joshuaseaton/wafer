// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

//! LEB128 decoding.

use core::ops;

// A LEB128-encodable integral type.
pub(super) trait Leb128:
    From<u8>                       //
    + ops::BitOrAssign             //
    + ops::Not<Output = Self>      //
    + ops::Shl<u32, Output = Self> //
{
    const MAX_BITS: u32;
    const IS_SIGNED: bool;
}

impl Leb128 for u32 {
    const MAX_BITS: u32 = 32;
    const IS_SIGNED: bool = false;
}

impl Leb128 for i32 {
    const MAX_BITS: u32 = 32;
    const IS_SIGNED: bool = true;
}

impl Leb128 for i64 {
    const MAX_BITS: u32 = 64;
    const IS_SIGNED: bool = true;
}

// Error trait for LEB128 parsing failures.
pub(super) trait Error {
    fn invalid_leb128() -> Self;
}

// Read a LEB128-encoded value using the provided byte source function.
//
// Implements LEB128 decoding per WASM specification. Validates encoding
// constraints including maximum length and proper unused bit handling.
pub(super) fn read<T, F, E>(mut read_byte: F) -> Result<T, E>
where
    T: Leb128,
    F: FnMut() -> Result<u8, E>,
    E: Error,
{
    const CONTENT_MASK: u8 = 0x7f;
    const LAST_CHUNK_MASK: u8 = 0x80;
    const SIGN_EXTEND_MASK: u8 = 0x40;

    let mut result = T::from(0);
    let mut shift = 0;
    let mut byte;

    loop {
        if shift >= T::MAX_BITS {
            return Err(E::invalid_leb128());
        }

        byte = read_byte()?;
        let content = byte & CONTENT_MASK;

        // Validate that the final byte doesn't overflow the remaining bits.
        if T::MAX_BITS - shift < 7 {
            let remaining_bits = T::MAX_BITS - shift;
            let valid = if T::IS_SIGNED {
                // For signed types, the unused bits must be consistent with the
                // sign bit.
                let mask = ((!0u8) << (remaining_bits - 1)) & CONTENT_MASK;
                let masked = content & mask;
                masked == 0 || masked == mask
            } else {
                // For unsigned types, the unused bits must be zero.
                content < (1u8 << remaining_bits)
            };
            if !valid {
                return Err(E::invalid_leb128());
            }
        }

        result |= T::from(content) << shift;
        shift += 7;

        if byte & LAST_CHUNK_MASK == 0 {
            break;
        }
    }

    // Sign extend if this is a signed type and the sign bit is set.
    if T::IS_SIGNED && shift < T::MAX_BITS && (byte & SIGN_EXTEND_MASK) != 0 {
        result |= !T::from(0) << shift;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum TestError {
        InvalidLeb128,
        Eof,
    }

    impl Error for TestError {
        fn invalid_leb128() -> Self {
            TestError::InvalidLeb128
        }
    }

    fn byte_reader(bytes: &[u8]) -> impl FnMut() -> Result<u8, TestError> + '_ {
        let mut index = 0;
        move || {
            if index >= bytes.len() {
                return Err(TestError::Eof);
            }
            let byte = bytes[index];
            index += 1;
            Ok(byte)
        }
    }

    fn read_u32(bytes: &[u8]) -> Result<u32, TestError> {
        read::<u32, _, _>(byte_reader(bytes))
    }

    fn read_i32(bytes: &[u8]) -> Result<i32, TestError> {
        read::<i32, _, _>(byte_reader(bytes))
    }

    fn read_i64(bytes: &[u8]) -> Result<i64, TestError> {
        read::<i64, _, _>(byte_reader(bytes))
    }

    #[test]
    fn test_u32_basic_values() {
        // Single byte values.
        assert_eq!(read_u32(&[0x00]), Ok(0));
        assert_eq!(read_u32(&[0x01]), Ok(1));
        assert_eq!(read_u32(&[0x7f]), Ok(127));

        // Two byte values.
        assert_eq!(read_u32(&[0x80, 0x01]), Ok(0x80));
        assert_eq!(read_u32(&[0xff, 0x01]), Ok(0xff));
        assert_eq!(read_u32(&[0x80, 0x02]), Ok(0x100));

        // Larger values.
        assert_eq!(read_u32(&[0x80, 0x80, 0x04]), Ok(0x10000));
        assert_eq!(read_u32(&[0xff, 0xff, 0xff, 0xff, 0x0f]), Ok(u32::MAX));
    }

    #[test]
    fn test_u32_non_minimal_valid() {
        // Non-minimal but valid encodings (padded with leading zeros).
        assert_eq!(read_u32(&[0x80, 0x00]), Ok(0x0));
        assert_eq!(read_u32(&[0x82, 0x00]), Ok(0x2));
        assert_eq!(read_u32(&[0x82, 0x80, 0x80, 0x80, 0x00]), Ok(0x2));
    }

    #[test]
    fn test_u32_integer_representation_too_long() {
        // More than 5 bytes for u32.
        assert_eq!(
            read_u32(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x00]),
            Err(TestError::InvalidLeb128)
        );
        assert_eq!(
            read_u32(&[0x82, 0x80, 0x80, 0x80, 0x80, 0x00]),
            Err(TestError::InvalidLeb128)
        );
    }

    #[test]
    fn test_u32_integer_too_large() {
        // Unused bits set.
        assert_eq!(
            read_u32(&[0x82, 0x80, 0x80, 0x80, 0x70]),
            Err(TestError::InvalidLeb128)
        );

        // Some unused bits set.
        assert_eq!(
            read_u32(&[0x82, 0x80, 0x80, 0x80, 0x40]),
            Err(TestError::InvalidLeb128)
        );

        // Single unused bit set.
        assert_eq!(
            read_u32(&[0x82, 0x80, 0x80, 0x80, 0x10]),
            Err(TestError::InvalidLeb128)
        );
    }

    #[test]
    fn test_i32_basic_values() {
        // Positive values.
        assert_eq!(read_i32(&[0x00]), Ok(0));
        assert_eq!(read_i32(&[0x01]), Ok(1));
        assert_eq!(read_i32(&[0x7f]), Ok(-1));

        // Negative values.
        assert_eq!(read_i32(&[0xff, 0x7f]), Ok(-1));
        assert_eq!(read_i32(&[0x80, 0x7f]), Ok(-128));
    }

    #[test]
    fn test_i32_non_minimal_valid() {
        // Non-minimal positive.
        assert_eq!(read_i32(&[0x80, 0x80, 0x80, 0x80, 0x00]), Ok(0));

        // Non-minimal negative.
        assert_eq!(read_i32(&[0xff, 0xff, 0xff, 0xff, 0x7f]), Ok(-1));
    }

    #[test]
    fn test_i32_integer_representation_too_long() {
        // More than 5 bytes for i32.
        assert_eq!(
            read_i32(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x00]),
            Err(TestError::InvalidLeb128)
        );
        assert_eq!(
            read_i32(&[0xff, 0xff, 0xff, 0xff, 0xff, 0x7f]),
            Err(TestError::InvalidLeb128)
        );
    }

    #[test]
    fn test_i32_integer_too_large() {
        // Unused bits not properly set for 0.
        assert_eq!(
            read_i32(&[0x80, 0x80, 0x80, 0x80, 0x70]),
            Err(TestError::InvalidLeb128)
        );

        // Unused bits not properly set for -1.
        assert_eq!(
            read_i32(&[0xff, 0xff, 0xff, 0xff, 0x0f]),
            Err(TestError::InvalidLeb128)
        );

        // Some unused bits wrong for 0.
        assert_eq!(
            read_i32(&[0x80, 0x80, 0x80, 0x80, 0x1f]),
            Err(TestError::InvalidLeb128)
        );

        // Some unused bits wrong for -1.
        assert_eq!(
            read_i32(&[0xff, 0xff, 0xff, 0xff, 0x4f]),
            Err(TestError::InvalidLeb128)
        );
    }

    #[test]
    fn test_i64_basic_values() {
        // Basic positive values
        assert_eq!(read_i64(&[0x00]), Ok(0));
        assert_eq!(read_i64(&[0x01]), Ok(1));

        // Basic negative values
        assert_eq!(read_i64(&[0x7f]), Ok(-1));
        assert_eq!(read_i64(&[0xff, 0x7f]), Ok(-1));
    }

    #[test]
    fn test_i64_non_minimal_valid() {
        // Non-minimal encodings with proper sign extension
        assert_eq!(read_i64(&[0x80, 0x00]), Ok(0));
        assert_eq!(read_i64(&[0xff, 0x7f]), Ok(-1));

        // Maximum length valid encodings
        assert_eq!(
            read_i64(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00]),
            Ok(0)
        );
        assert_eq!(
            read_i64(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f]),
            Ok(-1)
        );
    }

    #[test]
    fn test_i64_integer_representation_too_long() {
        // More than 10 bytes for i64
        assert_eq!(
            read_i64(&[
                0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x00
            ]),
            Err(TestError::InvalidLeb128)
        );
        assert_eq!(
            read_i64(&[
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f
            ]),
            Err(TestError::InvalidLeb128)
        );
    }

    #[test]
    fn test_i64_integer_too_large() {
        // Unused bits set for 0.
        assert_eq!(
            read_i64(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x7e]),
            Err(TestError::InvalidLeb128)
        );

        // Unused bits unset for -1.
        assert_eq!(
            read_i64(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01]),
            Err(TestError::InvalidLeb128)
        );

        // Some unused bits set for 0.
        assert_eq!(
            read_i64(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02]),
            Err(TestError::InvalidLeb128)
        );

        // Some unused bits unset for -1.
        assert_eq!(
            read_i64(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x41]),
            Err(TestError::InvalidLeb128)
        );
    }

    #[test]
    fn test_incomplete_encoding() {
        // Incomplete encodings (missing final byte)
        assert_eq!(read_u32(&[0x80]), Err(TestError::Eof));
        assert_eq!(read_u32(&[0x80, 0x80]), Err(TestError::Eof));
        assert_eq!(read_i32(&[0x80]), Err(TestError::Eof));
        assert_eq!(read_i64(&[0x80, 0x80, 0x80]), Err(TestError::Eof));
    }

    #[test]
    fn test_edge_case_boundaries() {
        // u32: Values around 32-bit boundary
        assert_eq!(read_u32(&[0xff, 0xff, 0xff, 0xff, 0x0f]), Ok(u32::MAX));
        assert_eq!(
            read_u32(&[0xff, 0xff, 0xff, 0xff, 0x1f]), // One bit too many
            Err(TestError::InvalidLeb128)
        );

        // i32: Maximum positive and minimum negative values
        assert_eq!(read_i32(&[0xff, 0xff, 0xff, 0xff, 0x07]), Ok(i32::MAX));
        assert_eq!(read_i32(&[0x80, 0x80, 0x80, 0x80, 0x78]), Ok(i32::MIN));

        // i64: Maximum valid values
        assert_eq!(
            read_i64(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00]),
            Ok(i64::MAX)
        );
        assert_eq!(
            read_i64(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x7f]),
            Ok(i64::MIN)
        );
    }

    #[test]
    fn test_zero_with_different_lengths() {
        // Zero encoded with different valid lengths.
        assert_eq!(read_u32(&[0x00]), Ok(0));
        assert_eq!(read_u32(&[0x80, 0x00]), Ok(0));
        assert_eq!(read_u32(&[0x80, 0x80, 0x00]), Ok(0));
        assert_eq!(read_u32(&[0x80, 0x80, 0x80, 0x00]), Ok(0));
        assert_eq!(read_u32(&[0x80, 0x80, 0x80, 0x80, 0x00]), Ok(0));

        // Same for signed.
        assert_eq!(read_i32(&[0x00]), Ok(0));
        assert_eq!(read_i32(&[0x80, 0x00]), Ok(0));
        assert_eq!(read_i32(&[0x80, 0x80, 0x80, 0x80, 0x00]), Ok(0));
    }

    #[test]
    fn test_negative_one_with_different_lengths() {
        // -1 encoded with different valid lengths.
        assert_eq!(read_i32(&[0x7f]), Ok(-1));
        assert_eq!(read_i32(&[0xff, 0x7f]), Ok(-1));
        assert_eq!(read_i32(&[0xff, 0xff, 0x7f]), Ok(-1));
        assert_eq!(read_i32(&[0xff, 0xff, 0xff, 0x7f]), Ok(-1));
        assert_eq!(read_i32(&[0xff, 0xff, 0xff, 0xff, 0x7f]), Ok(-1));

        // Same for i64.
        assert_eq!(read_i64(&[0x7f]), Ok(-1));
        assert_eq!(read_i64(&[0xff, 0x7f]), Ok(-1));
        assert_eq!(
            read_i64(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f]),
            Ok(-1)
        );
    }
}
