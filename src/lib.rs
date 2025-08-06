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
