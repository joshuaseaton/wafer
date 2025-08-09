// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

//! WebAssembly binary format parsing library.

#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![cfg_attr(nightly, feature(allocator_api))]

#[cfg(nightly)]
extern crate alloc;

pub mod core_compat;
pub mod decode;
pub mod storage;
pub mod types;

use core_compat::alloc::Allocator;
use decode::{ContextStack, CustomSectionVisitor, ErrorWithContext, decode_module};
use storage::Stream;
use types::{
    CodeSection, DataSection, ElementSection, ExportSection, FunctionSection, GlobalSection,
    ImportSection, MemorySection, TableSection, TypeSection, Version, for_each_bulk_opcode,
    for_each_opcode,
};

/// A WebAssembly module.
pub struct Module<A: Allocator + Clone> {
    /// Module version.
    pub version: Version,
    /// Function type declarations.
    pub typesec: Option<TypeSection<A>>,
    /// Import declarations.
    pub importsec: Option<ImportSection<A>>,
    /// Function type indices.
    pub funcsec: Option<FunctionSection<A>>,
    /// Table declarations.
    pub tablesec: Option<TableSection<A>>,
    /// Memory declarations.
    pub memsec: Option<MemorySection<A>>,
    /// Global variable declarations.
    pub globalsec: Option<GlobalSection<A>>,
    /// Export declarations.
    pub exportsec: Option<ExportSection<A>>,
    /// Start function index.
    pub startsec: Option<u32>,
    /// Element segments.
    pub elemsec: Option<ElementSection<A>>,
    /// Data segment count (for bulk memory operations).
    pub datacountsec: Option<u32>,
    /// Function bodies.
    pub codesec: Option<CodeSection<A>>,
    /// Data segments.
    pub datasec: Option<DataSection<A>>,
}

impl<A: Allocator + Clone> Module<A> {
    /// Decodes the module from streaming storage, with a given allocator and a
    /// custom section visitor.
    pub fn decode<Storage: Stream, CustomSecVisitor: CustomSectionVisitor<A>>(
        storage: Storage,
        customsec_visitor: &mut CustomSecVisitor,
        alloc: A,
    ) -> Result<Self, decode::ErrorWithContext<Storage>> {
        let mut context = ContextStack::default();
        decode_module(storage, &mut context, customsec_visitor, alloc)
            .map_err(|error| decode::ErrorWithContext { error, context })
    }
}
