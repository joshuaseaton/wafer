// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

use std::fs;

use spec_test_macro::wasm_spec_tests;
use wafer::Module;
use wafer::core_compat::alloc;
use wafer::decode::{self, NoCustomSectionVisitor};
use wafer::storage::MemoryEof;
use wafer::validate;

#[allow(unused)]
fn check_module(wasm: &str) {
    let bytes = fs::read(wasm).unwrap();
    let module =
        Module::decode_bytes(bytes, &mut NoCustomSectionVisitor {}, alloc::Global).unwrap();

    module.validate().unwrap();
}

#[allow(unused)]
fn assert_malformed(wasm: &str, expected: &wast2json::Error) {
    use wast2json::Error::*;

    let bytes = fs::read(wasm).unwrap();
    let result = Module::decode_bytes(bytes, &mut NoCustomSectionVisitor {}, alloc::Global);

    if let Err(error) = &result {
        let error = &error.error;

        macro_rules! error_matches {
            ($pattern:pat) => {
                assert!(matches!(error, $pattern), "Unexpected error: {error:?}")
            };
        }

        macro_rules! error_is {
            ($value:expr) => {
                assert_eq!(*error, $value, "Unexpected error: {error:?}")
            };
        }

        // Very much best-effort.
        match expected {
            EndOpcodeExpected => error_matches!(
                decode::Error::Storage(MemoryEof {})
                    | decode::Error::InvalidFunctionLength {
                        expected: _,
                        actual: _
                    }
            ),
            IllegalOpcode
            | MalformedImportKind
            | MalformedMutability
            | MalformedReferenceType
            | MalformedSectionId
            | ZeroByteExpected => error_matches!(decode::Error::InvalidToken(_)),
            IntegerRepresentationTooLong | IntegerTooLarge => {
                error_matches!(decode::Error::InvalidLeb128 | decode::Error::InvalidToken(_));
            }
            LengthOutOfBounds => {
                error_matches!(decode::Error::Storage(MemoryEof {}));
            }
            MagicHeaderNotDetected => error_matches!(decode::Error::InvalidMagic(_)),
            MalformedUtf8Encoding => error_is!(decode::Error::InvalidUtf8),
            SectionSizeMismatch => {
                error_matches!(
                    decode::Error::InvalidSectionLength {
                        id: _,
                        expected: _,
                        actual: _
                    } | decode::Error::InvalidFunctionLength {
                        expected: _,
                        actual: _
                    }
                );
            }
            TooManyLocals => error_matches!(decode::Error::TooManyLocals(_)),
            UnexpectedContentAfterLastSection => {
                error_matches!(decode::Error::OutOfOrderSection {
                    before: _,
                    after: _
                });
            }
            UnexpectedEnd | UnexpectedEndOfSectionOrFunction => {
                error_is!(decode::Error::Storage(MemoryEof {}));
            }
            UnknownBinaryVersion => error_matches!(decode::Error::UnknownVersion(_)),
            _ => todo!(
                "Handle wast2json::Error::{:?} -> wafer::decode::Error mapping",
                expected
            ),
        }
        return;
    }

    // If there's any remaining malformedness, it should be caught during
    // validation.
    let module = result.unwrap();
    let result = module.validate();
    let Err(error) = result else {
        panic!("Success!? Expected decoding or validation error: {expected:?}")
    };

    macro_rules! error_matches {
        ($pattern:pat) => {
            assert!(matches!(error, $pattern), "Unexpected error: {error:?}")
        };
    }

    macro_rules! error_is {
        ($value:expr) => {
            assert_eq!(error, $value, "Unexpected error: {error:?}")
        };
    }

    // Also very much best-effort.
    match expected {
        DataCountAndDataSectionHaveInconsistentLengths => {
            error_matches!(validate::Error::DataCountMismatch {
                expected: _,
                actual: _,
            });
        }
        FunctionAndCodeSectionHaveInconsistentLengths => {
            error_matches!(validate::Error::FunctionAndCodeSectionMismatch {
                funcsec_size: _,
                codesec_size: _
            });
        }
        _ => todo!(
            "Handle wast2json::Error::{:?} -> wafer::validate::Error mapping",
            expected
        ),
    }
}

wasm_spec_tests!();
