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

use core::fmt;

use decode::{ContextStack, CustomSectionVisitor, decode_module};
use storage::Stream;
use types::{
    CodeSection, DataSection, ElementSection, ExportSection, FunctionSection, GlobalSection,
    ImportSection, MemorySection, StartSection, TableSection, TypeSection, Version,
};

/// A convenience trait that captures the required allocation-related trait
/// bounds.
pub trait Allocator: core_compat::alloc::Allocator + fmt::Debug + Clone {}

impl<A> Allocator for A where A: core_compat::alloc::Allocator + fmt::Debug + Clone {}

/// A WebAssembly module.
pub struct Module<A: Allocator> {
    /// Module version.
    pub version: Version,
    /// Function type declarations.
    pub typesec: TypeSection<A>,
    /// Import declarations.
    pub importsec: ImportSection<A>,
    /// Function type indices.
    pub funcsec: FunctionSection<A>,
    /// Table declarations.
    pub tablesec: TableSection<A>,
    /// Memory declarations.
    pub memsec: MemorySection<A>,
    /// Global variable declarations.
    pub globalsec: GlobalSection<A>,
    /// Export declarations.
    pub exportsec: ExportSection<A>,
    /// Start function index.
    pub startsec: Option<StartSection>,
    /// Element segments.
    pub elemsec: ElementSection<A>,
    /// Data segment count (for bulk memory operations).
    pub datacountsec: Option<u32>,
    /// Function bodies.
    pub codesec: CodeSection<A>,
    /// Data segments.
    pub datasec: DataSection<A>,
}

impl<A: Allocator> Module<A> {
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
