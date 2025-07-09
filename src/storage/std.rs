// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

use std::io;

use super::Stream;

/// Stream implementation for `std::io` types.
impl<R: io::Read + io::Seek> Stream for R {
    type Error = io::Error;

    fn is_eof(err: &Self::Error) -> bool {
        err.kind() == io::ErrorKind::UnexpectedEof
    }

    fn offset(&mut self) -> usize {
        self.stream_position().unwrap().try_into().unwrap()
    }

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        io::Read::read_exact(self, buf)
    }

    fn skip_bytes(&mut self, count: usize) -> Result<(), Self::Error> {
        io::Seek::seek_relative(self, count.try_into().unwrap())
    }
}
