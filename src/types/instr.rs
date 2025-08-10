// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

//! WebAssembly instruction opcodes.
//!
//! Defines the opcodes for all WebAssembly instructions as specified in the
//! WebAssembly specification section 5.4.

use num_enum::TryFromPrimitive;

use crate::Allocator;
use crate::core_compat::vec::Vec;
use crate::{for_each_bulk_opcode, for_each_opcode};

use super::{ElemIdx, LabelIdx, TableIdx, TypeIdx, ValType};

// Operand type lookup tables for efficient instruction parsing
pub(crate) const OPCODE_TO_OPERAND_TYPE: [OperandType; 256] = {
    let mut table = [OperandType::None; 256];

    macro_rules! set {
        ($op:path, $type:expr) => {
            table[$op as usize] = $type;
        };
    }
    macro_rules! set_operand_type {
        (Opcode::Block) => {
            set!(Opcode::Block, OperandType::BlockType);
        };
        (Opcode::Br) => {
            set!(Opcode::Br, OperandType::U32);
        };
        (Opcode::BrIf) => {
            set!(Opcode::BrIf, OperandType::U32);
        };
        (Opcode::BrTable) => {
            set!(Opcode::BrTable, OperandType::BrTable);
        };
        (Opcode::Call) => {
            set!(Opcode::Call, OperandType::U32);
        };
        (Opcode::CallIndirect) => {
            set!(Opcode::CallIndirect, OperandType::CallIndirect);
        };
        (Opcode::F32Const) => {
            set!(Opcode::F32Const, OperandType::F32);
        };
        (Opcode::F32Load) => {
            set!(Opcode::F32Load, OperandType::MemArg);
        };
        (Opcode::F32Store) => {
            set!(Opcode::F32Store, OperandType::MemArg);
        };
        (Opcode::F64Const) => {
            set!(Opcode::F64Const, OperandType::F64);
        };
        (Opcode::F64Load) => {
            set!(Opcode::F64Load, OperandType::MemArg);
        };
        (Opcode::F64Store) => {
            set!(Opcode::F64Store, OperandType::MemArg);
        };
        (Opcode::GlobalGet) => {
            set!(Opcode::GlobalGet, OperandType::U32);
        };
        (Opcode::GlobalSet) => {
            set!(Opcode::GlobalSet, OperandType::U32);
        };
        (Opcode::I32Load) => {
            set!(Opcode::I32Load, OperandType::MemArg);
        };
        (Opcode::I32Load16S) => {
            set!(Opcode::I32Load16S, OperandType::MemArg);
        };
        (Opcode::I32Load16U) => {
            set!(Opcode::I32Load16U, OperandType::MemArg);
        };
        (Opcode::I32Load8S) => {
            set!(Opcode::I32Load8S, OperandType::MemArg);
        };
        (Opcode::I32Load8U) => {
            set!(Opcode::I32Load8U, OperandType::MemArg);
        };
        (Opcode::I32Store) => {
            set!(Opcode::I32Store, OperandType::MemArg);
        };
        (Opcode::I32Store16) => {
            set!(Opcode::I32Store16, OperandType::MemArg);
        };
        (Opcode::I32Store8) => {
            set!(Opcode::I32Store8, OperandType::MemArg);
        };
        (Opcode::I32Const) => {
            set!(Opcode::I32Const, OperandType::I32);
        };
        (Opcode::I64Const) => {
            set!(Opcode::I64Const, OperandType::I64);
        };
        (Opcode::I64Load) => {
            set!(Opcode::I64Load, OperandType::MemArg);
        };
        (Opcode::I64Load16S) => {
            set!(Opcode::I64Load16S, OperandType::MemArg);
        };
        (Opcode::I64Load16U) => {
            set!(Opcode::I64Load16U, OperandType::MemArg);
        };
        (Opcode::I64Load32S) => {
            set!(Opcode::I64Load32S, OperandType::MemArg);
        };
        (Opcode::I64Load32U) => {
            set!(Opcode::I64Load32U, OperandType::MemArg);
        };
        (Opcode::I64Load8S) => {
            set!(Opcode::I64Load8S, OperandType::MemArg);
        };
        (Opcode::I64Load8U) => {
            set!(Opcode::I64Load8U, OperandType::MemArg);
        };
        (Opcode::I64Store) => {
            set!(Opcode::I64Store, OperandType::MemArg);
        };
        (Opcode::I64Store16) => {
            set!(Opcode::I64Store16, OperandType::MemArg);
        };
        (Opcode::I64Store32) => {
            set!(Opcode::I64Store32, OperandType::MemArg);
        };
        (Opcode::I64Store8) => {
            set!(Opcode::I64Store8, OperandType::MemArg);
        };
        (Opcode::If) => {
            set!(Opcode::If, OperandType::BlockType);
        };
        (Opcode::LocalGet) => {
            set!(Opcode::LocalGet, OperandType::U32);
        };
        (Opcode::LocalSet) => {
            set!(Opcode::LocalSet, OperandType::U32);
        };
        (Opcode::LocalTee) => {
            set!(Opcode::LocalTee, OperandType::U32);
        };
        (Opcode::Loop) => {
            set!(Opcode::Loop, OperandType::BlockType);
        };
        (Opcode::RefFunc) => {
            set!(Opcode::RefFunc, OperandType::U32);
        };
        (Opcode::RefNull) => {
            set!(Opcode::RefNull, OperandType::RefType);
        };
        (Opcode::SelectT) => {
            set!(Opcode::SelectT, OperandType::SelectT);
        };
        (Opcode::TableGet) => {
            set!(Opcode::TableGet, OperandType::U32);
        };
        (Opcode::TableSet) => {
            set!(Opcode::TableSet, OperandType::U32);
        };
        (Opcode::BulkPrefix) => {
            set!(Opcode::BulkPrefix, OperandType::BulkOp);
        };
        (Opcode::VectorPrefix) => {
            set!(Opcode::VectorPrefix, OperandType::VectorOp);
        };
        ($other:path) => {};
    }
    for_each_opcode!(set_operand_type);
    table
};

pub(crate) const BULK_OPCODE_TO_OPERAND_TYPE: [BulkOperandType; 18] = {
    let mut table = [BulkOperandType::None; 18];

    macro_rules! set {
        ($op:path, $type:expr) => {
            table[$op as usize] = $type;
        };
    }
    macro_rules! set_bulk_operand_type {
        (BulkOpcode::DataDrop) => {
            set!(BulkOpcode::DataDrop, BulkOperandType::U32);
        };
        (BulkOpcode::ElemDrop) => {
            set!(BulkOpcode::ElemDrop, BulkOperandType::U32);
        };
        (BulkOpcode::MemoryInit) => {
            set!(BulkOpcode::MemoryInit, BulkOperandType::U32);
        };
        (BulkOpcode::TableCopy) => {
            set!(BulkOpcode::TableCopy, BulkOperandType::TableCopyOperands);
        };
        (BulkOpcode::TableFill) => {
            set!(BulkOpcode::TableFill, BulkOperandType::U32);
        };
        (BulkOpcode::TableGrow) => {
            set!(BulkOpcode::TableGrow, BulkOperandType::U32);
        };
        (BulkOpcode::TableInit) => {
            set!(BulkOpcode::TableInit, BulkOperandType::TableInitOperands);
        };
        (BulkOpcode::TableSize) => {
            set!(BulkOpcode::TableSize, BulkOperandType::U32);
        };
        ($other:path) => {};
    }
    for_each_bulk_opcode!(set_bulk_operand_type);
    table
};

/// Operand types for WebAssembly instructions
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OperandType {
    None,
    BlockType,
    BrTable,
    BulkOp,
    CallIndirect,
    F32,
    F64,
    I32,
    I64,
    MemArg,
    RefType,
    SelectT,
    U32,
    VectorOp,
}

// TODO(https://github.com/rust-lang/rust/issues/73662): Inline
// core::mem::variant_count::<OperandType>() instead of referencing a magic
// "14" here.
pub(crate) const OPERAND_TYPE_VARIANT_COUNT: usize = 14;

// Operand types for bulk memory/table instructions
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BulkOperandType {
    None,
    TableCopyOperands,
    TableInitOperands,
    U32,
}

// TODO(https://github.com/rust-lang/rust/issues/73662): Inline
// core::mem::variant_count::<BulkOperandType>() instead of referencing a magic
// "4" here.
pub(crate) const BULK_OPERAND_TYPE_VARIANT_COUNT: usize = 4;

/// Block type for control instructions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub enum BlockType {
    /// Block produces no results.
    Empty,
    /// Block produces a single result of the given type.
    Result(ValType),
    /// Block type is defined by function signature at given type index.
    TypeIndex(TypeIdx),
}

/// Operands for the `br_table` instruction.
#[derive(Debug)]
pub struct BrTableOperands<A: Allocator> {
    /// The vector of target labels for the `br_table` instruction.
    pub labels: Vec<LabelIdx, A>,
    /// The default label to branch to if the index is out of bounds.
    pub default: LabelIdx,
}

/// Operands for the `call_indirect` instruction.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CallIndirectOperands {
    /// Index of the table containing function references.
    pub table: TableIdx,
    /// Type index specifying the expected function signature.
    pub ty: TypeIdx,
}

/// Memory access operands for load and store instructions.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemArg {
    /// Static offset added to the dynamic address.
    pub offset: u32,
    /// Alignment hint (log2 of the alignment requirement).
    pub align: u32,
}

/// Operands for the typed `select` instruction.
#[derive(Debug)]
pub struct SelectTOperands<A: Allocator> {
    /// The vector of value types that the select instruction operates on.
    pub types: Vec<ValType, A>,
}

/// Operands for the `table.copy` instruction.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TableCopyOperands {
    /// Index of the source table.
    pub src: TableIdx,
    /// Index of the destination table.
    pub dst: TableIdx,
}

/// Operands for the `table.init` instruction.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TableInitOperands {
    /// Index of the table to initialize.
    pub table: TableIdx,
    /// Index of the element segment to use for initialization.
    pub elem: ElemIdx,
}

// [wasm]: 5.4.1 Control Instructions
//
/// WebAssembly instruction opcode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum Opcode {
    Unreachable = 0x00,
    Nop = 0x01,
    Block = 0x02,
    Loop = 0x03,
    If = 0x04,
    Else = 0x05,
    End = 0x0b,
    Br = 0x0c,
    BrIf = 0x0d,
    BrTable = 0x0e,
    Return = 0x0f,
    Call = 0x10,
    CallIndirect = 0x11,

    // [wasm]: 5.4.2 Reference Instructions
    RefNull = 0xd0,
    RefIsNull = 0xd1,
    RefFunc = 0xd2,

    // [wasm]: 5.4.3 Parametric Instructions
    Drop = 0x1a,
    Select = 0x1b,
    SelectT = 0x1c,

    // [wasm]: 5.4.4 Variable Instructions
    LocalGet = 0x20,
    LocalSet = 0x21,
    LocalTee = 0x22,
    GlobalGet = 0x23,
    GlobalSet = 0x24,

    // [wasm]: 5.4.5 Table Instructions
    TableGet = 0x25,
    TableSet = 0x26,

    // [wasm]: 5.4.6 Memory Instructions
    I32Load = 0x28,
    I64Load = 0x29,
    F32Load = 0x2a,
    F64Load = 0x2b,
    I32Load8S = 0x2c,
    I32Load8U = 0x2d,
    I32Load16S = 0x2e,
    I32Load16U = 0x2f,
    I64Load8S = 0x30,
    I64Load8U = 0x31,
    I64Load16S = 0x32,
    I64Load16U = 0x33,
    I64Load32S = 0x34,
    I64Load32U = 0x35,
    I32Store = 0x36,
    I64Store = 0x37,
    F32Store = 0x38,
    F64Store = 0x39,
    I32Store8 = 0x3a,
    I32Store16 = 0x3b,
    I64Store8 = 0x3c,
    I64Store16 = 0x3d,
    I64Store32 = 0x3e,
    MemorySize = 0x3f,
    MemoryGrow = 0x40,

    // [wasm]: 5.4.7 Numeric Instructions
    I32Const = 0x41,
    I64Const = 0x42,
    F32Const = 0x43,
    F64Const = 0x44,
    I32Eqz = 0x45,
    I32Eq = 0x46,
    I32Ne = 0x47,
    I32LtS = 0x48,
    I32LtU = 0x49,
    I32GtS = 0x4a,
    I32GtU = 0x4b,
    I32LeS = 0x4c,
    I32LeU = 0x4d,
    I32GeS = 0x4e,
    I32GeU = 0x4f,
    I64Eqz = 0x50,
    I64Eq = 0x51,
    I64Ne = 0x52,
    I64LtS = 0x53,
    I64LtU = 0x54,
    I64GtS = 0x55,
    I64GtU = 0x56,
    I64LeS = 0x57,
    I64LeU = 0x58,
    I64GeS = 0x59,
    I64GeU = 0x5a,
    F32Eq = 0x5b,
    F32Ne = 0x5c,
    F32Lt = 0x5d,
    F32Gt = 0x5e,
    F32Le = 0x5f,
    F32Ge = 0x60,
    F64Eq = 0x61,
    F64Ne = 0x62,
    F64Lt = 0x63,
    F64Gt = 0x64,
    F64Le = 0x65,
    F64Ge = 0x66,
    I32Clz = 0x67,
    I32Ctz = 0x68,
    I32Popcnt = 0x69,
    I32Add = 0x6a,
    I32Sub = 0x6b,
    I32Mul = 0x6c,
    I32DivS = 0x6d,
    I32DivU = 0x6e,
    I32RemS = 0x6f,
    I32RemU = 0x70,
    I32And = 0x71,
    I32Or = 0x72,
    I32Xor = 0x73,
    I32Shl = 0x74,
    I32ShrS = 0x75,
    I32ShrU = 0x76,
    I32Rotl = 0x77,
    I32Rotr = 0x78,
    I64Clz = 0x79,
    I64Ctz = 0x7a,
    I64Popcnt = 0x7b,
    I64Add = 0x7c,
    I64Sub = 0x7d,
    I64Mul = 0x7e,
    I64DivS = 0x7f,
    I64DivU = 0x80,
    I64RemS = 0x81,
    I64RemU = 0x82,
    I64And = 0x83,
    I64Or = 0x84,
    I64Xor = 0x85,
    I64Shl = 0x86,
    I64ShrS = 0x87,
    I64ShrU = 0x88,
    I64Rotl = 0x89,
    I64Rotr = 0x8a,
    F32Abs = 0x8b,
    F32Neg = 0x8c,
    F32Ceil = 0x8d,
    F32Floor = 0x8e,
    F32Trunc = 0x8f,
    F32Nearest = 0x90,
    F32Sqrt = 0x91,
    F32Add = 0x92,
    F32Sub = 0x93,
    F32Mul = 0x94,
    F32Div = 0x95,
    F32Min = 0x96,
    F32Max = 0x97,
    F32Copysign = 0x98,
    F64Abs = 0x99,
    F64Neg = 0x9a,
    F64Ceil = 0x9b,
    F64Floor = 0x9c,
    F64Trunc = 0x9d,
    F64Nearest = 0x9e,
    F64Sqrt = 0x9f,
    F64Add = 0xa0,
    F64Sub = 0xa1,
    F64Mul = 0xa2,
    F64Div = 0xa3,
    F64Min = 0xa4,
    F64Max = 0xa5,
    F64Copysign = 0xa6,
    I32WrapI64 = 0xa7,
    I32TruncF32S = 0xa8,
    I32TruncF32U = 0xa9,
    I32TruncF64S = 0xaa,
    I32TruncF64U = 0xab,
    I64ExtendI32S = 0xac,
    I64ExtendI32U = 0xad,
    I64TruncF32S = 0xae,
    I64TruncF32U = 0xaf,
    I64TruncF64S = 0xb0,
    I64TruncF64U = 0xb1,
    F32ConvertI32S = 0xb2,
    F32ConvertI32U = 0xb3,
    F32ConvertI64S = 0xb4,
    F32ConvertI64U = 0xb5,
    F32DemoteF64 = 0xb6,
    F64ConvertI32S = 0xb7,
    F64ConvertI32U = 0xb8,
    F64ConvertI64S = 0xb9,
    F64ConvertI64U = 0xba,
    F64PromoteF32 = 0xbb,
    I32ReinterpretF32 = 0xbc,
    I64ReinterpretF64 = 0xbd,
    F32ReinterpretI32 = 0xbe,
    F64ReinterpretI64 = 0xbf,
    I32Extend8S = 0xc0,
    I32Extend16S = 0xc1,
    I64Extend8S = 0xc2,
    I64Extend16S = 0xc3,
    I64Extend32S = 0xc4,

    // [wasm]: 5.4.7 Numeric Instructions
    // [wasm]: 5.4.5 Table Instructions
    //
    // Prefix for the The bulk memory and table instruction.
    BulkPrefix = 0xfc,

    // [wasm]: 5.4.8 Vector Instructions
    VectorPrefix = 0xfd,
}

/// Bulk memory and table instruction opcodes (0xfc prefix).
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum BulkOpcode {
    // [wasm]: 5.4.5 Table Instructions
    TableInit = 12,
    ElemDrop = 13,
    TableCopy = 14,
    TableGrow = 15,
    TableSize = 16,
    TableFill = 17,

    // [wasm]: 5.4.6 Memory Instructions
    MemoryInit = 8,
    DataDrop = 9,
    MemoryCopy = 10,
    MemoryFill = 11,

    // [wasm]: 5.4.7 Numeric Instructions
    I32TruncSatF32S = 0,
    I32TruncSatF32U = 1,
    I32TruncSatF64S = 2,
    I32TruncSatF64U = 3,
    I64TruncSatF32S = 4,
    I64TruncSatF32U = 5,
    I64TruncSatF64S = 6,
    I64TruncSatF64U = 7,
}

/// SIMD vector instruction opcodes (0xfd prefix).
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum VectorOpcode {
    V128Load = 0,
    V128Load8x8S = 1,
    V128Load8x8U = 2,
    V128Load16x4S = 3,
    V128Load16x4U = 4,
    V128Load32x2S = 5,
    V128Load32x2U = 6,
    V128Load8Splat = 7,
    V128Load16Splat = 8,
    V128Load32Splat = 9,
    V128Load64Splat = 10,
    V128Store = 11,
    V128Load32Zero = 92,
    V128Load64Zero = 93,

    V128Load8Lane = 84,
    V128Load16Lane = 85,
    V128Load32Lane = 86,
    V128Load64Lane = 87,
    V128Store8Lane = 88,
    V128Store16Lane = 89,
    V128Store32Lane = 90,
    V128Store64Lane = 91,

    V128Const = 12,

    I8x16Shuffle = 13,

    I8x16Swizzle = 14,

    I8x16Splat = 15,
    I16x8Splat = 16,
    I32x4Splat = 17,
    I64x2Splat = 18,
    F32x4Splat = 19,
    F64x2Splat = 20,

    I8x16ExtractLaneS = 21,
    I8x16ExtractLaneU = 22,
    I8x16ReplaceLane = 23,
    I16x8ExtractLaneS = 24,
    I16x8ExtractLaneU = 25,
    I16x8ReplaceLane = 26,
    I32x4ExtractLane = 27,
    I32x4ReplaceLane = 28,
    I64x2ExtractLane = 29,
    I64x2ReplaceLane = 30,
    F32x4ExtractLane = 31,
    F32x4ReplaceLane = 32,
    F64x2ExtractLane = 33,
    F64x2ReplaceLane = 34,

    I8x16Eq = 35,
    I8x16Ne = 36,
    I8x16LtS = 37,
    I8x16LtU = 38,
    I8x16GtS = 39,
    I8x16GtU = 40,
    I8x16LeS = 41,
    I8x16LeU = 42,
    I8x16GeS = 43,
    I8x16GeU = 44,

    I16x8Eq = 45,
    I16x8Ne = 46,
    I16x8LtS = 47,
    I16x8LtU = 48,
    I16x8GtS = 49,
    I16x8GtU = 50,
    I16x8LeS = 51,
    I16x8LeU = 52,
    I16x8GeS = 53,
    I16x8GeU = 54,

    I32x4Eq = 55,
    I32x4Ne = 56,
    I32x4LtS = 57,
    I32x4LtU = 58,
    I32x4GtS = 59,
    I32x4GtU = 60,
    I32x4LeS = 61,
    I32x4LeU = 62,
    I32x4GeS = 63,
    I32x4GeU = 64,

    I64x2Eq = 214,
    I64x2Ne = 215,
    I64x2LtS = 216,
    I64x2GtS = 217,
    I64x2LeS = 218,
    I64x2GeS = 219,

    F32x4Eq = 65,
    F32x4Ne = 66,
    F32x4Lt = 67,
    F32x4Gt = 68,
    F32x4Le = 69,
    F32x4Ge = 70,

    F64x2Eq = 71,
    F64x2Ne = 72,
    F64x2Lt = 73,
    F64x2Gt = 74,
    F64x2Le = 75,
    F64x2Ge = 76,

    V128Not = 77,
    V128And = 78,
    V128Andnot = 79,
    V128Or = 80,
    V128Xor = 81,
    V128Bitselect = 82,
    V128AnyTrue = 83,

    I8x16Abs = 96,
    I8x16Neg = 97,
    I8x16Popcnt = 98,
    I8x16AllTrue = 99,
    I8x16Bitmask = 100,
    I8x16NarrowI16x8S = 101,
    I8x16NarrowI16x8U = 102,
    I8x16Shl = 107,
    I8x16ShrS = 108,
    I8x16ShrU = 109,
    I8x16Add = 110,
    I8x16AddSatS = 111,
    I8x16AddSatU = 112,
    I8x16Sub = 113,
    I8x16SubSatS = 114,
    I8x16SubSatU = 115,
    I8x16MinS = 118,
    I8x16MinU = 119,
    I8x16MaxS = 120,
    I8x16MaxU = 121,
    I8x16AvgrU = 123,

    I16x8ExtaddPairwiseI8x16S = 124,
    I16x8ExtaddPairwiseI8x16U = 125,
    I16x8Abs = 128,
    I16x8Neg = 129,
    I16x8Q15mulrSatS = 130,
    I16x8AllTrue = 131,
    I16x8Bitmask = 132,
    I16x8NarrowI32x4S = 133,
    I16x8NarrowI32x4U = 134,
    I16x8ExtendLowI8x16S = 135,
    I16x8ExtendHighI8x16S = 136,
    I16x8ExtendLowI8x16U = 137,
    I16x8ExtendHighI8x16U = 138,
    I16x8Shl = 139,
    I16x8ShrS = 140,
    I16x8ShrU = 141,
    I16x8Add = 142,
    I16x8AddSatS = 143,
    I16x8AddSatU = 144,
    I16x8Sub = 145,
    I16x8SubSatS = 146,
    I16x8SubSatU = 147,
    I16x8Mul = 149,
    I16x8MinS = 150,
    I16x8MinU = 151,
    I16x8MaxS = 152,
    I16x8MaxU = 153,
    I16x8AvgrU = 155,
    I16x8ExtmulLowI8x16S = 156,
    I16x8ExtmulHighI8x16S = 157,
    I16x8ExtmulLowI8x16U = 158,
    I16x8ExtmulHighI8x16U = 159,

    I32x4ExtaddPairwiseI16x8S = 126,
    I32x4ExtaddPairwiseI16x8U = 127,
    I32x4Abs = 160,
    I32x4Neg = 161,
    I32x4AllTrue = 163,
    I32x4Bitmask = 164,
    I32x4ExtendLowI16x8S = 167,
    I32x4ExtendHighI16x8S = 168,
    I32x4ExtendLowI16x8U = 169,
    I32x4ExtendHighI16x8U = 170,
    I32x4Shl = 171,
    I32x4ShrS = 172,
    I32x4ShrU = 173,
    I32x4Add = 174,
    I32x4Sub = 177,
    I32x4Mul = 181,
    I32x4MinS = 182,
    I32x4MinU = 183,
    I32x4MaxS = 184,
    I32x4MaxU = 185,
    I32x4DotI16x8S = 186,
    I32x4ExtmulLowI16x8S = 188,
    I32x4ExtmulHighI16x8S = 189,
    I32x4ExtmulLowI16x8U = 190,
    I32x4ExtmulHighI16x8U = 191,

    I64x2Abs = 192,
    I64x2Neg = 193,
    I64x2AllTrue = 195,
    I64x2Bitmask = 196,
    I64x2ExtendLowI32x4S = 199,
    I64x2ExtendHighI32x4S = 200,
    I64x2ExtendLowI32x4U = 201,
    I64x2ExtendHighI32x4U = 202,
    I64x2Shl = 203,
    I64x2ShrS = 204,
    I64x2ShrU = 205,
    I64x2Add = 206,
    I64x2Sub = 209,
    I64x2Mul = 213,
    I64x2ExtmulLowI32x4S = 220,
    I64x2ExtmulHighI32x4S = 221,
    I64x2ExtmulLowI32x4U = 222,
    I64x2ExtmulHighI32x4U = 223,

    F32x4Ceil = 103,
    F32x4Floor = 104,
    F32x4Trunc = 105,
    F32x4Nearest = 106,
    F32x4Abs = 224,
    F32x4Neg = 225,
    F32x4Sqrt = 227,
    F32x4Add = 228,
    F32x4Sub = 229,
    F32x4Mul = 230,
    F32x4Div = 231,
    F32x4Min = 232,
    F32x4Max = 233,
    F32x4Pmin = 234,
    F32x4Pmax = 235,

    F64x2Ceil = 116,
    F64x2Floor = 117,
    F64x2Trunc = 122,
    F64x2Nearest = 148,
    F64x2Abs = 236,
    F64x2Neg = 237,
    F64x2Sqrt = 239,
    F64x2Add = 240,
    F64x2Sub = 241,
    F64x2Mul = 242,
    F64x2Div = 243,
    F64x2Min = 244,
    F64x2Max = 245,
    F64x2Pmin = 246,
    F64x2Pmax = 247,

    I32x4TruncSatF32x4S = 248,
    I32x4TruncSatF32x4U = 249,
    F32x4ConvertI32x4S = 250,
    F32x4ConvertI32x4U = 251,
    I32x4TruncSatF64x2SZero = 252,
    I32x4TruncSatF64x2UZero = 253,
    F64x2ConvertLowI32x4S = 254,
    F64x2ConvertLowI32x4U = 255,
    F32x4DemoteF64x2Zero = 94,
    F64x2PromoteLowF32x4 = 95,
}
