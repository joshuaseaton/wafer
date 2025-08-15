// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

//! Storage abstraction for sequential binary data reading.
//!
//! Provides the [`Stream`] trait for reading binary data sequentially,
//! with implementations for in-memory buffers and standard I/O types.

#[cfg(feature = "std")]
mod std;

use core::fmt;

/// Storage abstraction for the streamed reading of a WASM module.
pub trait Stream {
    /// Error type for storage-specific failures.
    type Error: fmt::Debug;

    /// Whether the given error signifies a failure to read due to having
    /// reached the end of the stream (i.e., the "End Of the File").
    fn is_eof(err: &Self::Error) -> bool;

    /// Returns the current byte offset into the stream.
    fn offset(&mut self) -> usize;

    /// Reads a single byte from the stream.
    fn read_byte(&mut self) -> Result<u8, Self::Error>;

    /// Reads exactly `buf.len()` bytes into the provided buffer.
    ///
    /// Returns an error if EOF is reached or an I/O error occurs
    /// before the buffer is completely filled.
    ///
    /// Implementors should override for better performance.
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        for byte in buf {
            *byte = self.read_byte()?;
        }
        Ok(())
    }

    /// Skip the specified number of bytes in the stream.
    ///
    /// Implementors should override for better performance.
    fn skip_bytes(&mut self, count: usize) -> Result<(), Self::Error> {
        for _ in 0..count {
            self.read_byte()?;
        }
        Ok(())
    }
}

/// Represents attempting to read past the end of a buffer.
#[derive(Debug)]
pub struct MemoryEof {}

/// In-memory buffer implementation of [`Stream`].
pub(super) struct Buffer<Bytes: AsRef<[u8]>> {
    bytes: Bytes,
    pos: usize,
}

impl<Bytes: AsRef<[u8]>> Buffer<Bytes> {
    /// Create a new buffer stream from the given bytes.
    pub(super) fn new(bytes: Bytes) -> Self {
        Self { bytes, pos: 0 }
    }
}

impl<Bytes: AsRef<[u8]>> Stream for Buffer<Bytes> {
    type Error = MemoryEof;

    fn is_eof(_: &Self::Error) -> bool {
        true
    }

    fn offset(&mut self) -> usize {
        self.pos
    }

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        let bytes = self.bytes.as_ref();
        if self.pos < bytes.len() {
            let byte = bytes[self.pos];
            self.pos += 1;
            Ok(byte)
        } else {
            Err(MemoryEof {})
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        let bytes = self.bytes.as_ref();
        debug_assert!(self.pos <= bytes.len());
        if buf.len() <= bytes.len() - self.pos {
            buf.copy_from_slice(&bytes[self.pos..(self.pos + buf.len())]);
            self.pos += buf.len();
            Ok(())
        } else {
            Err(MemoryEof {})
        }
    }

    fn skip_bytes(&mut self, count: usize) -> Result<(), Self::Error> {
        let bytes = self.bytes.as_ref();
        debug_assert!(self.pos <= bytes.len());
        if count <= bytes.len() - self.pos {
            self.pos += count;
            Ok(())
        } else {
            Err(MemoryEof {})
        }
    }
}
