// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

//! WebAssembly type definitions.
//!
//! This module contains all the WebAssembly type definitions used for parsing
//! and representing WASM modules, including value types, function signatures,
//! imports, exports, and other WASM constructs.

mod instr;
pub use instr::*;

use core::cmp;

use num_enum::TryFromPrimitive;

use crate::Allocator;
use crate::core_compat::boxed::Box;
use crate::core_compat::vec::Vec;

// Defines a public newtype without public mutable access to the underlying
// type, along with some convenience trait implementations like Deref and AsRef.
macro_rules! newtype {
    (
        $(#[$meta:meta])*
        pub struct $type:ident<$($lifetime:lifetime, )? A: Allocator>($underlying:ty);
    ) => {
        $(#[$meta])*
        pub struct $type<$($lifetime, )? A: Allocator>($underlying);

        newtype!(@impl [$($lifetime, )? A: Allocator], $type<$($lifetime, )? A>, $underlying);
    };
    (
        $(#[$meta:meta])*
        pub struct $type:ident($underlying:ty);
    ) => {
        $(#[$meta])*
        pub struct $type($underlying);

        newtype!(@impl [], $type, $underlying);
    };
    (
        $(#[$meta:meta])*
        pub struct $type:ident<$lifetime:lifetime>($underlying:ty);
    ) => {
        $(#[$meta])*
        pub struct $type<$lifetime>($underlying);

        newtype!(@impl [$lifetime], $type<$lifetime>, $underlying);
    };
    (@impl [$($generic_params:tt)*], $qualified_type:ty, $underlying:ty) => {
        impl<$($generic_params)*> $qualified_type {
            pub fn new(value: $underlying) -> Self {
                Self(value)
            }
        }

        impl<$($generic_params)*> ::core::ops::Deref for $qualified_type {
            type Target = $underlying;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<$($generic_params)*> AsRef<$underlying> for $qualified_type {
            fn as_ref(&self) -> &$underlying {
                &self.0
            }
        }
    };
}
pub(crate) use newtype;

/// WebAssembly module version.
#[derive(Clone, Copy, Debug, TryFromPrimitive)]
#[repr(u32)]
pub enum Version {
    V1 = 1,
}

newtype!(
    /// A name (of a module, section, or field).
    #[derive(Debug, Eq, PartialEq)]
    pub struct Name<A: Allocator>(Box<str, A>);
);

/// The type of a reference to an object in the runtime store.
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum RefType {
    /// Function reference type.
    Func = 0x70,
    /// External reference type.
    Extern = 0x6f,
}

/// Value types classify the individual values that WebAssembly code can compute
/// with and the values that a variable accepts.
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum ValType {
    /// 32-bit signed integer.
    I32 = 0x7f,
    /// 64-bit signed integer.
    I64 = 0x7e,
    /// 32-bit floating point number.
    F32 = 0x7d,
    /// 64-bit floating point number.
    F64 = 0x7c,
    /// 128-bit SIMD vector.
    Vec = 0x7b,
    /// Function reference.
    FuncRef = RefType::Func as u8,
    /// External reference.
    ExternRef = RefType::Extern as u8,
}

newtype!(
    /// The sequence of types representing the result of executing instructions
    /// or functions.
    #[derive(Debug, Clone)]
    pub struct ResultType<A: Allocator>(Vec<ValType, A>);
);

/// The signature of a function, mapping parameters to results. They are also
/// used to classify the inputs and outputs of instructions.
#[derive(Clone, Debug)]
pub struct FunctionType<A: Allocator> {
    pub parameters: Vec<ValType, A>,
    pub results: ResultType<A>,
}

/// The size range of the resizeable storage associated with memory (# of pages)
/// and table types (# of elements).
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Minimum size.
    pub min: u32,
    /// Maximum size, with None indicating that there is no upper limit.
    pub max: Option<u32>,
}

newtype!(
    /// A linear memory type with its size limits.
    #[derive(Clone, Copy, Debug)]
    pub struct MemType(Limits);
);

impl MemType {
    /// The WebAssembly page size.
    pub const PAGE_SIZE: usize = 0x1_0000; // 64 KiB

    /// The minimum size in bytes of the linear memory region.
    pub const fn min_size_bytes(&self) -> usize {
        (self.0.min as usize) * Self::PAGE_SIZE
    }

    /// The maximum size in bytes of the linear memory region, if any.
    pub fn max_size_bytes(&self) -> Option<usize> {
        self.0.max.map(|max| (max as usize) * Self::PAGE_SIZE)
    }
}

/// WebAssembly table type.
#[derive(Clone, Copy, Debug)]
pub struct TableType {
    /// Type of references stored in table.
    pub reftype: RefType,
    /// Table size limits.
    pub limits: Limits,
}

impl TableType {
    /// The minimum number of elements in the table.
    pub const fn min_elements(&self) -> u32 {
        self.limits.min
    }

    /// The maximum number of elements in the table, if any.
    pub const fn max_elements(&self) -> Option<u32> {
        self.limits.max
    }
}

/// The mutability of a global variable.
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum GlobalTypeMutability {
    /// Immutable.
    Const = 0x00,
    /// Mutable.
    Var = 0x01,
}

/// Represents a global variable.
#[derive(Clone, Copy, Debug)]
pub struct GlobalType {
    /// The type of the global.
    pub value: ValType,

    /// The mutability of the global.
    pub mutability: GlobalTypeMutability,
}

newtype!(
    /// An index into the type section.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct TypeIdx(u32);
);

newtype!(
    /// An index into the function section.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct FuncIdx(u32);
);

newtype!(
    /// An index into the table section.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct TableIdx(u32);
);

newtype!(
    /// An index into the memory section.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct MemIdx(u32);
);

newtype!(
    /// An index into the global section.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct GlobalIdx(u32);
);

newtype!(
    /// An index into the element section.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct ElemIdx(u32);
);

newtype!(
    /// An index into the data section.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct DataIdx(u32);
);

newtype!(
    /// An index into a function's local variables.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct LocalIdx(u32);
);

newtype!(
    /// An index referencing structured control instructions inside an
    /// instruction sequence.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct LabelIdx(u32);
);

newtype!(
    /// Represents a WebAssembly bytecode expression, but re-encoded in a way
    /// specific to the crate:
    /// * opcodes remain unchanged;
    /// * fixed-size operands are encoded in their repr(C) representations in
    ///   this module, along natural alignments (padded out with zeroes); in
    ///   particular, integers are encoded as little endian and not LEB128;
    /// * vector operands remain encoded as a u32 count followed by the sequence
    ///   of elements, but the count and elements are encoded per the previous
    ///   point;
    /// * reserved zero bytes are stripped
    ///
    ///  The re-encodings along natural alignments are meant to make the
    ///  execution of this code more efficient.
    #[derive(Clone, Debug)]
    pub struct Expression<A: Allocator>(Box<[u8], A>);
);

/// Section identifier within a module.
///
/// `PartialOrd` is implemented so that, for non-custom section IDs, an ID is
/// less than another precisely when the former has must appear in a module
/// before the latter in a module when both are present.
#[derive(Clone, Copy, Debug, Eq, TryFromPrimitive, PartialEq)]
#[repr(u8)]
pub enum SectionId {
    /// Custom section with arbitrary data.
    Custom = 0,
    /// Function type declarations.
    Type = 1,
    /// Import declarations.
    Import = 2,
    /// Function type indices for module functions.
    Function = 3,
    /// Table declarations.
    Table = 4,
    /// Memory declarations.
    Memory = 5,
    /// Global declarations.
    Global = 6,
    /// Export declarations.
    Export = 7,
    /// Start function index.
    Start = 8,
    /// Element segments for table initialization.
    Element = 9,
    /// Function bodies.
    Code = 10,
    /// Data segments for memory initialization.
    Data = 11,
    /// Data segment count (for bulk memory operations).
    DataCount = 12,
}

// The logical order, as documented above.
impl PartialOrd for SectionId {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        use SectionId::{Code, Data, DataCount};

        // Handle the special case where DataCount can appear before Code
        match (self, other) {
            // DataCount comes before Code and Data in the logical ordering
            (DataCount, Code | Data) => Some(cmp::Ordering::Less),
            (Code | Data, DataCount) => Some(cmp::Ordering::Greater),

            // For all other cases, use the numeric ordering
            _ => (*self as u8).partial_cmp(&(*other as u8)),
        }
    }
}

/// Custom section with arbitrary binary data.
pub struct CustomSection<A: Allocator> {
    /// Section name.
    pub name: Name<A>,
    /// Section content.
    pub bytes: Box<[u8], A>,
}

newtype!(
    /// Section containing function type declarations.
    #[derive(Clone, Debug)]
    pub struct TypeSection<A: Allocator>(Vec<FunctionType<A>, A>);
);

/// Import descriptor types.
#[derive(Clone, Copy, Debug)]
pub enum ImportDescriptor {
    /// Import a function with the given type index.
    Function(TypeIdx),
    /// Import a table with the given type.
    Table(TableType),
    /// Import a memory with the given type.
    Memory(MemType),
    /// Import a global with the given type.
    Global(GlobalType),
}

/// An import declaration.
#[derive(Debug)]
pub struct Import<A: Allocator> {
    /// Name of the module to import from.
    pub module: Name<A>,
    /// Name of the imported entity.
    pub field: Name<A>,
    /// Type of the imported entity.
    pub descriptor: ImportDescriptor,
}

newtype!(
    /// Section containing import declarations.
    #[derive(Debug)]
    pub struct ImportSection<A: Allocator>(Vec<Import<A>, A>);
);

newtype!(
    /// Section containing type indices for module-defined functions.
    #[derive(Clone, Debug)]
    pub struct FunctionSection<A: Allocator>(Vec<TypeIdx, A>);
);

newtype!(
    /// Section containing table type declarations.
    #[derive(Clone, Debug)]
    pub struct TableSection<A: Allocator>(Vec<TableType, A>);
);

newtype!(
    /// Section containing linear memory type declarations.
    #[derive(Clone, Debug)]
    pub struct MemorySection<A: Allocator>(Vec<MemType, A>);
);

/// A global declaration.
#[derive(Clone, Debug)]
pub struct Global<A: Allocator> {
    /// Global type and mutability.
    pub ty: GlobalType,
    /// Initialization expression.
    pub init: Expression<A>,
}

newtype!(
    /// Section containing global variable declarations.
    #[derive(Clone, Debug)]
    pub struct GlobalSection<A: Allocator>(Vec<Global<A>, A>);
);

/// Describes what kind of entity is being exported.
#[derive(Clone, Copy, Debug)]
pub enum ExportDescriptor {
    /// Export a function with the given index.
    Function(FuncIdx),
    /// Export a table with the given index.
    Table(TableIdx),
    /// Export a memory with the given index.
    Memory(MemIdx),
    /// Export a global with the given index.
    Global(GlobalIdx),
}

/// An export declaration.
#[derive(Debug)]
pub struct Export<A: Allocator> {
    /// Name of the exported entity.
    pub field: Name<A>,
    /// Type and index of the exported entity.
    pub descriptor: ExportDescriptor,
}

newtype!(
    /// Section containing export declarations.
    #[derive(Debug)]
    pub struct ExportSection<A: Allocator>(Vec<Export<A>, A>);
);

newtype!(
    /// Holds the index of the start function.
    #[derive(Clone, Copy, Debug)]
    pub struct StartSection(FuncIdx);
);

// [wasm]: 5.5.12 Element Section

newtype!(
    /// Section containing element segments for table initialization.
    #[derive(Clone, Debug)]
    pub struct ElementSection<A: Allocator>(Vec<ElementSegment<A>, A>);
);

/// WebAssembly element segment.
#[derive(Clone, Debug)]
pub struct ElementSegment<A: Allocator> {
    /// The type of references this element segment contains.
    pub ty: RefType,
    /// The initial values for the element segment.
    pub init: ElementInit<A>,
    /// How this element segment should be placed (active, passive, or
    /// declarative).
    pub mode: ElementMode<A>,
}

/// The initialization data for an element segment.
#[derive(Clone, Debug)]
pub enum ElementInit<A: Allocator> {
    /// Element segment contains function indices.
    FunctionIndices(Vec<FuncIdx, A>),
    /// Element segment contains initialization expressions.
    Expressions(Vec<Expression<A>, A>),
}

/// Active element mode with table and offset.
#[derive(Clone, Debug)]
pub struct ElementModeActive<A: Allocator> {
    /// Index of the table to initialize.
    pub table: TableIdx,
    /// Expression computing the offset within the table.
    pub offset: Expression<A>,
}

/// Element segment mode.
#[derive(Clone, Debug)]
pub enum ElementMode<A: Allocator> {
    Passive,
    Active(ElementModeActive<A>),
    Declarative,
}

/// A local variable with its type and initial value.
#[derive(Clone, Copy, Debug)]
pub enum Local {
    /// 32-bit integer local variable.
    I32(i32),
    /// 64-bit integer local variable.
    I64(i64),
    /// 32-bit floating point local variable.
    F32(f32),
    /// 64-bit floating point local variable.
    F64(f64),
    /// Function reference local variable.
    FuncRef(u32),
    // TODO: Vec, ExternRef
}

newtype!(
    /// Collection of local variables for a function.
    #[derive(Debug)]
    pub struct Locals<A: Allocator>(Vec<Local, A>);
);

/// A WebAssembly function with its local variables and bytecode.
#[derive(Debug)]
pub struct Function<A: Allocator> {
    /// Local variable declarations for this function.
    pub locals: Locals<A>,
    /// The function's compiled bytecode expression.
    pub code: Expression<A>,
}

newtype!(
    /// Section containing function bodies.
    #[derive(Debug)]
    pub struct CodeSection<A: Allocator>(Vec<Function<A>, A>);
);

/// A data segment for initializing linear memory.
#[derive(Debug)]
pub struct DataSegment<A: Allocator> {
    /// The initial data bytes for this segment.
    pub init: Vec<u8, A>,
    /// How this data segment should be placed (active or passive).
    pub mode: DataMode<A>,
}

/// The placement mode for a data segment.
#[derive(Debug)]
pub enum DataMode<A: Allocator> {
    /// Passive data segment (must be explicitly copied via memory.init).
    Passive(),
    /// Active data segment (automatically copied to memory during instantiation).
    Active(DataModeActive<A>),
}

/// Active placement information for a data segment.
#[derive(Debug)]
pub struct DataModeActive<A: Allocator> {
    /// Index of the memory to initialize.
    pub memory: MemIdx,
    /// Expression computing the offset within the memory.
    pub offset: Expression<A>,
}

newtype!(
    /// Section containing data segments for memory initialization.
    #[derive(Debug)]
    pub struct DataSection<A: Allocator>(Vec<DataSegment<A>, A>);
);

// Invokes a callback declarative macro for each WebAssembly opcode.
macro_rules! for_each_opcode {
    ($callback:ident) => {
        $callback!(Opcode::Block);
        $callback!(Opcode::Br);
        $callback!(Opcode::BrIf);
        $callback!(Opcode::BrTable);
        $callback!(Opcode::BulkPrefix);
        $callback!(Opcode::Call);
        $callback!(Opcode::CallIndirect);
        $callback!(Opcode::Drop);
        $callback!(Opcode::Else);
        $callback!(Opcode::End);
        $callback!(Opcode::F32Abs);
        $callback!(Opcode::F32Add);
        $callback!(Opcode::F32Ceil);
        $callback!(Opcode::F32ConvertI32S);
        $callback!(Opcode::F32ConvertI32U);
        $callback!(Opcode::F32ConvertI64S);
        $callback!(Opcode::F32ConvertI64U);
        $callback!(Opcode::F32Const);
        $callback!(Opcode::F32Copysign);
        $callback!(Opcode::F32DemoteF64);
        $callback!(Opcode::F32Div);
        $callback!(Opcode::F32Eq);
        $callback!(Opcode::F32Floor);
        $callback!(Opcode::F32Ge);
        $callback!(Opcode::F32Gt);
        $callback!(Opcode::F32Le);
        $callback!(Opcode::F32Load);
        $callback!(Opcode::F32Lt);
        $callback!(Opcode::F32Max);
        $callback!(Opcode::F32Min);
        $callback!(Opcode::F32Mul);
        $callback!(Opcode::F32Ne);
        $callback!(Opcode::F32Nearest);
        $callback!(Opcode::F32Neg);
        $callback!(Opcode::F32ReinterpretI32);
        $callback!(Opcode::F32Sqrt);
        $callback!(Opcode::F32Store);
        $callback!(Opcode::F32Sub);
        $callback!(Opcode::F32Trunc);
        $callback!(Opcode::F64Abs);
        $callback!(Opcode::F64Add);
        $callback!(Opcode::F64Ceil);
        $callback!(Opcode::F64ConvertI32S);
        $callback!(Opcode::F64ConvertI32U);
        $callback!(Opcode::F64ConvertI64S);
        $callback!(Opcode::F64ConvertI64U);
        $callback!(Opcode::F64Const);
        $callback!(Opcode::F64Copysign);
        $callback!(Opcode::F64Div);
        $callback!(Opcode::F64Eq);
        $callback!(Opcode::F64Floor);
        $callback!(Opcode::F64Ge);
        $callback!(Opcode::F64Gt);
        $callback!(Opcode::F64Le);
        $callback!(Opcode::F64Load);
        $callback!(Opcode::F64Lt);
        $callback!(Opcode::F64Max);
        $callback!(Opcode::F64Min);
        $callback!(Opcode::F64Mul);
        $callback!(Opcode::F64Ne);
        $callback!(Opcode::F64Nearest);
        $callback!(Opcode::F64Neg);
        $callback!(Opcode::F64PromoteF32);
        $callback!(Opcode::F64ReinterpretI64);
        $callback!(Opcode::F64Sqrt);
        $callback!(Opcode::F64Store);
        $callback!(Opcode::F64Sub);
        $callback!(Opcode::F64Trunc);
        $callback!(Opcode::GlobalGet);
        $callback!(Opcode::GlobalSet);
        $callback!(Opcode::I32Add);
        $callback!(Opcode::I32And);
        $callback!(Opcode::I32Clz);
        $callback!(Opcode::I32Const);
        $callback!(Opcode::I32Ctz);
        $callback!(Opcode::I32DivS);
        $callback!(Opcode::I32DivU);
        $callback!(Opcode::I32Eq);
        $callback!(Opcode::I32Eqz);
        $callback!(Opcode::I32Extend16S);
        $callback!(Opcode::I32Extend8S);
        $callback!(Opcode::I32GeS);
        $callback!(Opcode::I32GeU);
        $callback!(Opcode::I32GtS);
        $callback!(Opcode::I32GtU);
        $callback!(Opcode::I32LeS);
        $callback!(Opcode::I32LeU);
        $callback!(Opcode::I32Load);
        $callback!(Opcode::I32Load16S);
        $callback!(Opcode::I32Load16U);
        $callback!(Opcode::I32Load8S);
        $callback!(Opcode::I32Load8U);
        $callback!(Opcode::I32LtS);
        $callback!(Opcode::I32LtU);
        $callback!(Opcode::I32Mul);
        $callback!(Opcode::I32Ne);
        $callback!(Opcode::I32Or);
        $callback!(Opcode::I32Popcnt);
        $callback!(Opcode::I32ReinterpretF32);
        $callback!(Opcode::I32RemS);
        $callback!(Opcode::I32RemU);
        $callback!(Opcode::I32Rotl);
        $callback!(Opcode::I32Rotr);
        $callback!(Opcode::I32Shl);
        $callback!(Opcode::I32ShrS);
        $callback!(Opcode::I32ShrU);
        $callback!(Opcode::I32Store);
        $callback!(Opcode::I32Store16);
        $callback!(Opcode::I32Store8);
        $callback!(Opcode::I32Sub);
        $callback!(Opcode::I32TruncF32S);
        $callback!(Opcode::I32TruncF32U);
        $callback!(Opcode::I32TruncF64S);
        $callback!(Opcode::I32TruncF64U);
        $callback!(Opcode::I32WrapI64);
        $callback!(Opcode::I32Xor);
        $callback!(Opcode::I64Add);
        $callback!(Opcode::I64And);
        $callback!(Opcode::I64Clz);
        $callback!(Opcode::I64Const);
        $callback!(Opcode::I64Ctz);
        $callback!(Opcode::I64DivS);
        $callback!(Opcode::I64DivU);
        $callback!(Opcode::I64Eq);
        $callback!(Opcode::I64Eqz);
        $callback!(Opcode::I64Extend16S);
        $callback!(Opcode::I64Extend32S);
        $callback!(Opcode::I64Extend8S);
        $callback!(Opcode::I64ExtendI32S);
        $callback!(Opcode::I64ExtendI32U);
        $callback!(Opcode::I64GeS);
        $callback!(Opcode::I64GeU);
        $callback!(Opcode::I64GtS);
        $callback!(Opcode::I64GtU);
        $callback!(Opcode::I64LeS);
        $callback!(Opcode::I64LeU);
        $callback!(Opcode::I64Load);
        $callback!(Opcode::I64Load16S);
        $callback!(Opcode::I64Load16U);
        $callback!(Opcode::I64Load32S);
        $callback!(Opcode::I64Load32U);
        $callback!(Opcode::I64Load8S);
        $callback!(Opcode::I64Load8U);
        $callback!(Opcode::I64LtS);
        $callback!(Opcode::I64LtU);
        $callback!(Opcode::I64Mul);
        $callback!(Opcode::I64Ne);
        $callback!(Opcode::I64Or);
        $callback!(Opcode::I64Popcnt);
        $callback!(Opcode::I64ReinterpretF64);
        $callback!(Opcode::I64RemS);
        $callback!(Opcode::I64RemU);
        $callback!(Opcode::I64Rotl);
        $callback!(Opcode::I64Rotr);
        $callback!(Opcode::I64Shl);
        $callback!(Opcode::I64ShrS);
        $callback!(Opcode::I64ShrU);
        $callback!(Opcode::I64Store);
        $callback!(Opcode::I64Store16);
        $callback!(Opcode::I64Store32);
        $callback!(Opcode::I64Store8);
        $callback!(Opcode::I64Sub);
        $callback!(Opcode::I64TruncF32S);
        $callback!(Opcode::I64TruncF32U);
        $callback!(Opcode::I64TruncF64S);
        $callback!(Opcode::I64TruncF64U);
        $callback!(Opcode::I64Xor);
        $callback!(Opcode::If);
        $callback!(Opcode::LocalGet);
        $callback!(Opcode::LocalSet);
        $callback!(Opcode::LocalTee);
        $callback!(Opcode::Loop);
        $callback!(Opcode::MemoryGrow);
        $callback!(Opcode::MemorySize);
        $callback!(Opcode::Nop);
        $callback!(Opcode::RefFunc);
        $callback!(Opcode::RefIsNull);
        $callback!(Opcode::RefNull);
        $callback!(Opcode::Return);
        $callback!(Opcode::Select);
        $callback!(Opcode::SelectT);
        $callback!(Opcode::TableGet);
        $callback!(Opcode::TableSet);
        $callback!(Opcode::Unreachable);
        $callback!(Opcode::VectorPrefix);
    };
}
pub(crate) use for_each_opcode;

// Invokes a callback declarative macro for each WebAssembly bulk opcode.
macro_rules! for_each_bulk_opcode {
    ($callback:ident) => {
        $callback!(BulkOpcode::DataDrop);
        $callback!(BulkOpcode::ElemDrop);
        $callback!(BulkOpcode::I32TruncSatF32S);
        $callback!(BulkOpcode::I32TruncSatF32U);
        $callback!(BulkOpcode::I32TruncSatF64S);
        $callback!(BulkOpcode::I32TruncSatF64U);
        $callback!(BulkOpcode::I64TruncSatF32S);
        $callback!(BulkOpcode::I64TruncSatF32U);
        $callback!(BulkOpcode::I64TruncSatF64S);
        $callback!(BulkOpcode::I64TruncSatF64U);
        $callback!(BulkOpcode::MemoryCopy);
        $callback!(BulkOpcode::MemoryFill);
        $callback!(BulkOpcode::MemoryInit);
        $callback!(BulkOpcode::TableCopy);
        $callback!(BulkOpcode::TableFill);
        $callback!(BulkOpcode::TableGrow);
        $callback!(BulkOpcode::TableInit);
        $callback!(BulkOpcode::TableSize);
    };
}
pub(crate) use for_each_bulk_opcode;
