// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

use std::fs::File;
use std::io;

use spec_test_macro::wasm_spec_tests;
use wafer::Module;
use wafer::core_compat::alloc;
use wafer::decode::{Error, NoCustomSectionVisitor};
use wafer::storage::Stream;

#[derive(Debug)]
struct BufReader<R>(io::BufReader<R>);

impl<R> PartialEq for BufReader<R> {
    fn eq(&self, _other: &Self) -> bool {
        // We don't need to actually compare BufReaders in tests
        true
    }
}

impl<R: io::Read> BufReader<R> {
    fn new(inner: R) -> Self {
        Self(io::BufReader::new(inner))
    }
}

impl<R: io::Read + io::Seek> Stream for BufReader<R> {
    type Error = io::ErrorKind;

    fn offset(&mut self) -> usize {
        self.0.offset()
    }

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        self.0.read_byte().map_err(|e| e.kind())
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.0.read_exact(buf).map_err(|e| e.kind())
    }

    fn skip_bytes(&mut self, count: usize) -> Result<(), Self::Error> {
        self.0.skip_bytes(count).map_err(|e| e.kind())
    }

    fn is_eof(err: &Self::Error) -> bool {
        matches!(err, io::ErrorKind::UnexpectedEof)
    }
}

#[allow(unused)]
fn check_module(wasm: &str) {
    let f = File::open(wasm).unwrap();
    Module::decode(
        io::BufReader::new(f),
        &mut NoCustomSectionVisitor {},
        alloc::Global,
    )
    .unwrap();
}

#[allow(unused)]
fn assert_malformed(wasm: &str, expected: &wast2json::Error) {
    use wast2json::Error::*;

    let f = File::open(wasm).unwrap();
    let result = Module::decode(
        BufReader::new(f),
        &mut NoCustomSectionVisitor {},
        alloc::Global,
    );

    let error = match result {
        Err(error) => error.error,
        Ok(_) => panic!("Success!? Expected error: {expected:?}"),
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

    // Very much best-effort.
    match expected {
        EndOpcodeExpected => error_matches!(
            Error::Storage(io::ErrorKind::UnexpectedEof)
                | Error::InvalidFunctionLength {
                    expected: _,
                    actual: _
                }
        ),
        IllegalOpcode
        | MalformedImportKind
        | MalformedMutability
        | MalformedReferenceType
        | MalformedSectionId
        | ZeroByteExpected => error_matches!(Error::InvalidToken(_)),
        IntegerRepresentationTooLong | IntegerTooLarge => {
            error_matches!(Error::InvalidLeb128 | Error::InvalidToken(_));
        }
        LengthOutOfBounds => error_matches!(Error::Storage(io::ErrorKind::UnexpectedEof)),
        MagicHeaderNotDetected => error_matches!(Error::InvalidMagic(_)),
        MalformedUtf8Encoding => error_is!(Error::InvalidUtf8),
        SectionSizeMismatch => {
            error_matches!(
                Error::InvalidSectionLength {
                    id: _,
                    expected: _,
                    actual: _
                } | Error::InvalidFunctionLength {
                    expected: _,
                    actual: _
                }
            );
        }
        TooManyLocals => error_matches!(Error::TooManyLocals(_)),
        UnexpectedContentAfterLastSection => error_matches!(Error::OutOfOrderSection {
            before: _,
            after: _
        }),
        UnexpectedEnd | UnexpectedEndOfSectionOrFunction => {
            error_is!(Error::Storage(io::ErrorKind::UnexpectedEof));
        }

        UnknownBinaryVersion => error_matches!(Error::UnknownVersion(_)),
        _ => todo!(
            "Handle wast2json::Error::{:?} -> decode error mapping",
            expected
        ),
    }
}

wasm_spec_tests!();
