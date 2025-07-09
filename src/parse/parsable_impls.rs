// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

//! Parsable trait implementations for WebAssembly types.

use core::ops;

use num_enum::TryFromPrimitive;

use crate::core_compat::alloc::Allocator;
use crate::core_compat::boxed::Box;
use crate::core_compat::vec::Vec;
use crate::storage::Stream;
use crate::types::*;

use super::{
    BoundedParsable, ContextId, ContextStack, Contextual, Error, InvalidLength, Magic, Parsable,
    Parser, transcode_expression,
};

/// Maximum number of local variables per function. It serves to give a
/// reasonable static upper bound, as the spec only gives an upper bound of
/// 2^32 - 1 (unrealistically large) and we need to allocate space for local
/// upfront.
const MAX_LOCALS_PER_FUNCTION: usize = 2000;

macro_rules! impl_contextual {
    ($type:ident<A: Allocator>, $id:path) => {
        impl<A: Allocator> Contextual for $type<A> {
            const ID: ContextId = $id;
        }
    };
    ($type:ident<A: Allocator>, $id:path) => {
        impl<A: Allocator> Contextual for $type<A> {
            const ID: ContextId = $id;
        }
    };
    (Vec<$type:ty, A>, $id:path) => {
        impl<A: Allocator> Contextual for Vec<$type, A> {
            const ID: ContextId = $id;
        }
    };
    ($type:ty, $id:path) => {
        impl Contextual for $type {
            const ID: ContextId = $id;
        }
    };
}

macro_rules! impl_parsable_for_u8_enum {
    ($type:ty) => {
        impl BoundedParsable for $type {
            fn parse<Storage: Stream>(
                parser: &mut Parser<Storage>,
                _: &mut ContextStack,
            ) -> Result<Self, Error<Storage>> {
                let byte = parser.read_byte_raw()?;
                Self::try_from(byte).map_err(|_| Error::InvalidToken(byte))
            }
        }
    };
}

macro_rules! impl_parsable_for_leb128_u32_enum {
    ($type:ty, $make_err:path) => {
        impl BoundedParsable for $type {
            fn parse<Storage: Stream>(
                parser: &mut Parser<Storage>,
                _: &mut ContextStack,
            ) -> Result<Self, Error<Storage>> {
                let val: u32 = parser.read_leb128_raw()?;
                Self::try_from(val).map_err(|_| $make_err(val))
            }
        }
    };
}

macro_rules! impl_parsable_for_le_u32_enum {
    ($type:ty, $make_err:path) => {
        impl BoundedParsable for $type {
            fn parse<Storage: Stream>(
                parser: &mut Parser<Storage>,
                _: &mut ContextStack,
            ) -> Result<Self, Error<Storage>> {
                let mut buf = [0u8; 4];
                parser.read_exact_raw(&mut buf)?;
                let val = u32::from_le_bytes(buf);
                Self::try_from(val).map_err(|_| $make_err(val))
            }
        }
    };
}

macro_rules! impl_parsable_for_newtype {
    ($type:ident<A>) => {
        impl<A: Allocator + Clone> Parsable<A> for $type<A> {
            fn parse<Storage: Stream>(
                parser: &mut Parser<Storage>,
                context: &mut ContextStack,
                alloc: &A,
            ) -> Result<Self, Error<Storage>> {
                Ok(Self::new(<Self as ops::Deref>::Target::parse(
                    parser, context, alloc,
                )?))
            }
        }
    };
    ($type:ident) => {
        impl BoundedParsable for $type {
            fn parse<Storage: Stream>(
                parser: &mut Parser<Storage>,
                context: &mut ContextStack,
            ) -> Result<Self, Error<Storage>> {
                Ok(Self::new(
                    <<Self as ops::Deref>::Target as BoundedParsable>::parse(parser, context)?,
                ))
            }
        }
    };
}

impl<T, A> Parsable<A> for Vec<T, A>
where
    T: Parsable<A> + Contextual,
    A: Allocator + Clone,
{
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<Self, Error<Storage>> {
        let mut len: u32 = parser.read_bounded(context)?;
        let mut vec = Vec::new_in(alloc.clone());
        vec.try_reserve_exact(len as usize)?;
        while len > 0 {
            vec.push(parser.read(context, alloc)?);
            len -= 1;
        }
        Ok(vec)
    }
}

impl_contextual!(i32, ContextId::I32);
impl_contextual!(i64, ContextId::I64);
impl_contextual!(f32, ContextId::F32);
impl_contextual!(f64, ContextId::F64);
impl_contextual!(BulkOpcode, ContextId::BulkOpcode);
impl_contextual!(BrTableOperands<A: Allocator>, ContextId::BrTableOperands);
impl_contextual!(CallIndirectOperands, ContextId::U32);
impl_contextual!(CodeSection<A: Allocator>, ContextId::CodeSec);
impl_contextual!(CustomSection<A: Allocator>, ContextId::CustomSec);
impl_contextual!(DataIdx, ContextId::DataIdx);
impl_contextual!(DataSection<A: Allocator>, ContextId::DataSec);
impl_contextual!(DataSegment<A: Allocator>, ContextId::Data);
impl_contextual!(DataSegmentToken, ContextId::DataToken);
impl_contextual!(ElemIdx, ContextId::ElemIdx);
impl_contextual!(ElementKind, ContextId::ElemKind);
impl_contextual!(ElementSection<A: Allocator>, ContextId::ElemSec);
impl_contextual!(ElementSegment<A: Allocator>, ContextId::Elem);
impl_contextual!(ElementSegmentToken, ContextId::ElemToken);
impl_contextual!(Export<A: Allocator>, ContextId::Export);
impl_contextual!(ExportDescriptor, ContextId::ExportDesc);
impl_contextual!(ExportDescriptorToken, ContextId::ExportDescToken);
impl_contextual!(ExportSection<A: Allocator>, ContextId::ExportSec);
impl_contextual!(Expression<A: Allocator>, ContextId::Expr);
impl_contextual!(Function<A: Allocator>, ContextId::Func);
impl_contextual!(FunctionSection<A: Allocator>, ContextId::FuncSec);
impl_contextual!(FunctionType<A: Allocator>, ContextId::FuncType);
impl_contextual!(FunctionTypeToken, ContextId::FuncTypeToken);
impl_contextual!(FuncIdx, ContextId::FuncIdx);
impl_contextual!(Global<A: Allocator>, ContextId::Global);
impl_contextual!(GlobalIdx, ContextId::GlobalIdx);
impl_contextual!(GlobalSection<A: Allocator>, ContextId::GlobalSec);
impl_contextual!(GlobalType, ContextId::GlobalType);
impl_contextual!(GlobalTypeMutability, ContextId::Mut);
impl_contextual!(Import<A: Allocator>, ContextId::Import);
impl_contextual!(ImportDescriptor, ContextId::ImportDesc);
impl_contextual!(ImportDescriptorToken, ContextId::ImportDescToken);
impl_contextual!(ImportSection<A: Allocator>, ContextId::ImportSec);
impl_contextual!(LabelIdx, ContextId::LabelIdx);
impl_contextual!(Limits, ContextId::Limits);
impl_contextual!(LimitsToken, ContextId::LimitsMaxToken);
impl_contextual!(LocalIdx, ContextId::LocalIdx);
impl_contextual!(Locals<A: Allocator>, ContextId::Locals);
impl_contextual!(Magic, ContextId::Magic);
impl_contextual!(MemArg, ContextId::MemArg);
impl_contextual!(MemIdx, ContextId::MemIdx);
impl_contextual!(MemorySection<A: Allocator>, ContextId::MemorySec);
impl_contextual!(MemType, ContextId::MemType);
impl_contextual!(Name<A: Allocator>, ContextId::Name);
impl_contextual!(Opcode, ContextId::Opcode);
impl_contextual!(RefType, ContextId::RefType);
impl_contextual!(ResultType<A: Allocator>, ContextId::ResultType);
impl_contextual!(SectionId, ContextId::SectionId);
impl_contextual!(SelectTOperands<A: Allocator>, ContextId::SelectTOperands);
impl_contextual!(TableCopyOperands, ContextId::U32);
impl_contextual!(TableIdx, ContextId::TableIdx);
impl_contextual!(TableInitOperands, ContextId::U32);
impl_contextual!(TableSection<A: Allocator>, ContextId::TableSec);
impl_contextual!(TableType, ContextId::TableType);
impl_contextual!(TypeIdx, ContextId::TypeIdx);
impl_contextual!(TypeSection<A: Allocator>, ContextId::TypeSec);
impl_contextual!(u32, ContextId::U32);
impl_contextual!(u8, ContextId::Byte);
impl_contextual!(ValType, ContextId::ValType);
impl_contextual!(Vec<u8, A>, ContextId::VecByte);
impl_contextual!(BlockType, ContextId::BlockType);
impl_contextual!(Vec<Function<A>, A>, ContextId::VecCode);
impl_contextual!(Vec<Expression<A>, A>, ContextId::VecExpr);
impl_contextual!(Vec<FuncIdx, A>, ContextId::VecFuncIdx);
impl_contextual!(Vec<LabelIdx, A>, ContextId::VecLabelIdx);
impl_contextual!(Vec<ValType, A>, ContextId::VecValType);
impl_contextual!(Version, ContextId::Version);

impl_parsable_for_u8_enum!(ElementKind);
impl_parsable_for_u8_enum!(ExportDescriptorToken);
impl_parsable_for_u8_enum!(FunctionTypeToken);
impl_parsable_for_u8_enum!(GlobalTypeMutability);
impl_parsable_for_u8_enum!(ImportDescriptorToken);
impl_parsable_for_u8_enum!(LimitsToken);
impl_parsable_for_u8_enum!(Opcode);
impl_parsable_for_u8_enum!(RefType);
impl_parsable_for_u8_enum!(SectionId);
impl_parsable_for_u8_enum!(ValType);

impl_parsable_for_leb128_u32_enum!(BulkOpcode, Error::InvalidBulkOpcode);
impl_parsable_for_leb128_u32_enum!(DataSegmentToken, Error::InvalidDataToken);
impl_parsable_for_leb128_u32_enum!(ElementSegmentToken, Error::InvalidElementToken);

impl_parsable_for_le_u32_enum!(Magic, Error::InvalidMagic);
impl_parsable_for_le_u32_enum!(Version, Error::UnknownVersion);

impl_parsable_for_newtype!(DataIdx);
impl_parsable_for_newtype!(ElemIdx);
impl_parsable_for_newtype!(FuncIdx);
impl_parsable_for_newtype!(GlobalIdx);
impl_parsable_for_newtype!(LabelIdx);
impl_parsable_for_newtype!(LocalIdx);
impl_parsable_for_newtype!(MemIdx);
impl_parsable_for_newtype!(MemType);
impl_parsable_for_newtype!(TableIdx);
impl_parsable_for_newtype!(TypeIdx);
impl_parsable_for_newtype!(CodeSection<A>);
impl_parsable_for_newtype!(DataSection<A>);
impl_parsable_for_newtype!(ElementSection<A>);
impl_parsable_for_newtype!(ExportSection<A>);
impl_parsable_for_newtype!(FunctionSection<A>);
impl_parsable_for_newtype!(GlobalSection<A>);
impl_parsable_for_newtype!(ImportSection<A>);
impl_parsable_for_newtype!(MemorySection<A>);
impl_parsable_for_newtype!(ResultType<A>);
impl_parsable_for_newtype!(TableSection<A>);
impl_parsable_for_newtype!(TypeSection<A>);

impl BoundedParsable for u8 {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        _: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        parser.read_byte_raw()
    }
}

impl BoundedParsable for u32 {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        _: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        parser.read_leb128_raw()
    }
}

impl BoundedParsable for i32 {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        _: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        parser.read_leb128_raw()
    }
}

impl BoundedParsable for i64 {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        _: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        parser.read_leb128_raw()
    }
}

impl BoundedParsable for f32 {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        _: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        let mut buf = [0u8; 4];
        parser.read_exact_raw(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }
}

impl BoundedParsable for f64 {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        _: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        let mut buf = [0u8; 8];
        parser.read_exact_raw(&mut buf)?;
        Ok(f64::from_le_bytes(buf))
    }
}

impl BoundedParsable for CallIndirectOperands {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        Ok(Self {
            table: parser.read_bounded(context)?,
            ty: parser.read_bounded(context)?,
        })
    }
}

impl BoundedParsable for MemArg {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        Ok(Self {
            align: parser.read_bounded(context)?,
            offset: parser.read_bounded(context)?,
        })
    }
}

impl BoundedParsable for TableCopyOperands {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        Ok(Self {
            src: parser.read_bounded(context)?,
            dst: parser.read_bounded(context)?,
        })
    }
}

impl BoundedParsable for TableInitOperands {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        Ok(Self {
            table: parser.read_bounded(context)?,
            elem: parser.read_bounded(context)?,
        })
    }
}

impl<A: Allocator + Clone> Parsable<A> for BrTableOperands<A> {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<Self, Error<Storage>> {
        Ok(Self {
            labels: parser.read(context, alloc)?,
            default: parser.read_bounded(context)?,
        })
    }
}

impl<A: Allocator + Clone> Parsable<A> for SelectTOperands<A> {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<Self, Error<Storage>> {
        Ok(Self {
            types: parser.read(context, alloc)?,
        })
    }
}

impl BoundedParsable for BlockType {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        let value: i32 = parser.read_bounded(context)?;
        match value {
            n if n < 0 => {
                // For single-byte values encoded as signed LEB128:
                // - Bytes 0x40-0x7F (64-127) become negative when interpreted as signed
                // - Conversion: signed_byte = original_byte - 128 (for bytes >= 64)
                // - Reverse: original_byte = signed_byte + 128
                let byte = u8::try_from(n + 128)
                    .expect("Signed LEB128 block type value must be convertible to single byte");

                if byte == 0x40 {
                    Ok(BlockType::Empty)
                } else {
                    match ValType::try_from(byte) {
                        Ok(valtype) => Ok(BlockType::Result(valtype)),
                        Err(_) => Err(Error::InvalidValType(byte)),
                    }
                }
            }
            n => Ok(BlockType::TypeIndex(u32::try_from(n).unwrap())),
        }
    }
}

impl<A: Allocator + Clone> Parsable<A> for Name<A> {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<Self, Error<Storage>> {
        let len: u32 = parser.read_bounded(context)?;
        let mut bytes = Vec::new_in(alloc.clone());
        bytes.try_reserve_exact(len as usize)?;
        // Safety: With the previous call, there is sufficient capacity and any
        // uninitialized bytes will be overwritten in the next call to
        // read_exact().
        unsafe { bytes.set_len(len as usize) };
        parser.read_exact(context, &mut bytes)?;

        str::from_utf8(&bytes).map_err(|_| Error::InvalidUtf8)?;
        let bytes_ptr = Box::into_raw(bytes.into_boxed_slice());

        // Safety: The ABIs of [u8] and str are identical, and we have already
        // validated that the byte sequence is valid UTF-8.
        let str = unsafe { Box::from_raw_in(bytes_ptr as *mut str, alloc.clone()) };
        Ok(Self::new(str))
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, TryFromPrimitive)]
enum FunctionTypeToken {
    Value = 0x60,
}

impl<A: Allocator + Clone> Parsable<A> for FunctionType<A> {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<Self, Error<Storage>> {
        parser.read_bounded::<FunctionTypeToken>(context)?;
        Ok(Self {
            parameters: parser.read(context, alloc)?,
            results: parser.read(context, alloc)?,
        })
    }
}

#[derive(Copy, Clone, TryFromPrimitive)]
#[repr(u8)]
enum LimitsToken {
    WithoutMax = 0x00,
    WithMax = 0x01,
}

impl BoundedParsable for Limits {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        let token: LimitsToken = parser.read_bounded(context)?;
        let min: u32 = parser.read_bounded(context)?;
        let max = match token {
            LimitsToken::WithoutMax => None,
            LimitsToken::WithMax => Some(parser.read_bounded(context)?),
        };
        Ok(Self { min, max })
    }
}

impl BoundedParsable for TableType {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        Ok(Self {
            reftype: parser.read_bounded(context)?,
            limits: parser.read_bounded(context)?,
        })
    }
}

impl BoundedParsable for GlobalType {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        Ok(Self {
            value: parser.read_bounded(context)?,
            mutability: parser.read_bounded(context)?,
        })
    }
}

impl<A: Allocator + Clone> Parsable<A> for Expression<A> {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<Self, Error<Storage>> {
        transcode_expression(parser, context, alloc)
    }
}

#[derive(TryFromPrimitive, Copy, Clone)]
#[repr(u8)]
enum ImportDescriptorToken {
    Function = 0x0,
    Table = 0x1,
    Memory = 0x2,
    Global = 0x3,
}
impl BoundedParsable for ImportDescriptor {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        type Token = ImportDescriptorToken;

        match parser.read_bounded(context)? {
            Token::Function => Ok(ImportDescriptor::Function(parser.read_bounded(context)?)),
            Token::Table => Ok(ImportDescriptor::Table(parser.read_bounded(context)?)),
            Token::Memory => Ok(ImportDescriptor::Memory(parser.read_bounded(context)?)),
            Token::Global => Ok(ImportDescriptor::Global(parser.read_bounded(context)?)),
        }
    }
}

impl<A: Allocator + Clone> Parsable<A> for Import<A> {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<Self, Error<Storage>> {
        Ok(Self {
            module: parser.read(context, alloc)?,
            field: parser.read(context, alloc)?,
            descriptor: parser.read_bounded(context)?,
        })
    }
}

impl<A: Allocator + Clone> Parsable<A> for Global<A> {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<Self, Error<Storage>> {
        Ok(Self {
            ty: parser.read_bounded(context)?,
            init: parser.read(context, alloc)?,
        })
    }
}

#[derive(TryFromPrimitive, Copy, Clone)]
#[repr(u8)]
enum ExportDescriptorToken {
    Function = 0x0,
    Table = 0x1,
    Memory = 0x2,
    Global = 0x3,
}
impl BoundedParsable for ExportDescriptor {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
    ) -> Result<Self, Error<Storage>> {
        type Token = ExportDescriptorToken;

        match parser.read_bounded(context)? {
            Token::Function => Ok(ExportDescriptor::Function(parser.read_bounded(context)?)),
            Token::Table => Ok(ExportDescriptor::Table(parser.read_bounded(context)?)),
            Token::Memory => Ok(ExportDescriptor::Memory(parser.read_bounded(context)?)),
            Token::Global => Ok(ExportDescriptor::Global(parser.read_bounded(context)?)),
        }
    }
}

impl<A: Allocator + Clone> Parsable<A> for Export<A> {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<Self, Error<Storage>> {
        Ok(Self {
            field: parser.read(context, alloc)?,
            descriptor: parser.read_bounded(context)?,
        })
    }
}

impl<A: Allocator + Clone> Parsable<A> for ElementSegment<A> {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<Self, Error<Storage>> {
        let token: ElementSegmentToken = parser.read_bounded(context)?;
        match token {
            ElementSegmentToken::ActiveElemIndices => {
                let active = ElementModeActive {
                    table: TableIdx::new(0),
                    offset: parser.read(context, alloc)?,
                };
                let funcs: Vec<FuncIdx, A> = parser.read(context, alloc)?;
                Ok(ElementSegment {
                    ty: RefType::Func,
                    init: ElementInit::FunctionIndices(funcs),
                    mode: ElementMode::Active(active),
                })
            }
            ElementSegmentToken::PassiveElemIndices => {
                let kind: ElementKind = parser.read_bounded(context)?;
                let funcs: Vec<FuncIdx, A> = parser.read(context, alloc)?;
                Ok(ElementSegment {
                    ty: kind.into(),
                    init: ElementInit::FunctionIndices(funcs),
                    mode: ElementMode::Passive,
                })
            }
            ElementSegmentToken::ActiveTableIndexElemIndices => {
                let active = ElementModeActive {
                    table: parser.read(context, alloc)?,
                    offset: parser.read(context, alloc)?,
                };
                let kind: ElementKind = parser.read_bounded(context)?;
                let funcs: Vec<FuncIdx, A> = parser.read(context, alloc)?;
                Ok(ElementSegment {
                    ty: kind.into(),
                    init: ElementInit::FunctionIndices(funcs),
                    mode: ElementMode::Active(active),
                })
            }
            ElementSegmentToken::DeclarativeElemIndices => {
                let kind: ElementKind = parser.read_bounded(context)?;
                let funcs: Vec<FuncIdx, A> = parser.read(context, alloc)?;
                Ok(ElementSegment {
                    ty: kind.into(),
                    init: ElementInit::FunctionIndices(funcs),
                    mode: ElementMode::Declarative,
                })
            }
            ElementSegmentToken::ActiveElemExprs => {
                let active = ElementModeActive {
                    table: TableIdx::new(0),
                    offset: parser.read(context, alloc)?,
                };
                let exprs: Vec<Expression<A>, A> = parser.read(context, alloc)?;
                Ok(ElementSegment {
                    ty: RefType::Func,
                    init: ElementInit::Expressions(exprs),
                    mode: ElementMode::Active(active),
                })
            }
            ElementSegmentToken::PassiveElemExprs => {
                let reftype: RefType = parser.read_bounded(context)?;
                let exprs: Vec<Expression<A>, A> = parser.read(context, alloc)?;
                Ok(ElementSegment {
                    ty: reftype,
                    init: ElementInit::Expressions(exprs),
                    mode: ElementMode::Passive,
                })
            }
            ElementSegmentToken::ActiveTableIndexElemExprs => {
                let active = ElementModeActive {
                    table: parser.read(context, alloc)?,
                    offset: parser.read(context, alloc)?,
                };
                let reftype: RefType = parser.read_bounded(context)?;
                let exprs: Vec<Expression<A>, A> = parser.read(context, alloc)?;
                Ok(ElementSegment {
                    ty: reftype,
                    init: ElementInit::Expressions(exprs),
                    mode: ElementMode::Active(active),
                })
            }
            ElementSegmentToken::DeclarativeElemExprs => {
                let reftype: RefType = parser.read_bounded(context)?;
                let exprs: Vec<Expression<A>, A> = parser.read(context, alloc)?;
                Ok(ElementSegment {
                    ty: reftype,
                    init: ElementInit::Expressions(exprs),
                    mode: ElementMode::Declarative,
                })
            }
        }
    }
}

#[derive(Copy, Clone, TryFromPrimitive)]
#[repr(u32)]
enum ElementSegmentToken {
    ActiveElemIndices = 0,
    PassiveElemIndices = 1,
    ActiveTableIndexElemIndices = 2,
    DeclarativeElemIndices = 3,
    ActiveElemExprs = 4,
    PassiveElemExprs = 5,
    ActiveTableIndexElemExprs = 6,
    DeclarativeElemExprs = 7,
}

#[derive(Copy, Clone, Debug, TryFromPrimitive)]
#[repr(u8)]
enum ElementKind {
    FuncRef = 0x00,
}

impl From<ElementKind> for RefType {
    fn from(value: ElementKind) -> Self {
        match value {
            ElementKind::FuncRef => Self::Func,
        }
    }
}

impl<A: Allocator + Clone> Parsable<A> for Locals<A> {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<Self, Error<Storage>> {
        let num_groups: u32 = parser.read_bounded(context)?;
        let mut locals = Vec::new_in(alloc.clone());
        for _ in 0..num_groups {
            let count: u32 = parser.read_bounded(context)?;
            let local = Local::from(parser.read_bounded::<ValType>(context)?);
            let subtotal = locals.len() + (count as usize);
            if subtotal > MAX_LOCALS_PER_FUNCTION {
                return Err(Error::TooManyLocals(subtotal));
            }
            locals.try_reserve_exact(count as usize)?;
            locals.resize(subtotal, local); // No allocation with previous reservation.
        }
        Ok(Locals::new(locals))
    }
}

impl From<ValType> for Local {
    fn from(value: ValType) -> Self {
        match value {
            ValType::I32 => Local::I32(0),
            ValType::I64 => Local::I64(0),
            ValType::F32 => Local::F32(0.0),
            ValType::F64 => Local::F64(0.0),
            ValType::FuncRef => Local::FuncRef(0),
            ValType::Vec | ValType::ExternRef => todo!(),
        }
    }
}

impl<A: Allocator + Clone> Parsable<A> for Function<A> {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<Self, Error<Storage>> {
        let expected_size = parser.read_bounded::<u32>(context)? as usize;
        let offset_start = parser.offset();
        let locals = parser.read(context, alloc)?;
        let code = parser.read(context, alloc)?;
        let actual_size = parser.offset() - offset_start;
        if expected_size != actual_size {
            return Err(Error::InvalidFunctionLength(InvalidLength {
                expected: expected_size as u32,
                actual: actual_size as u32,
            }));
        }
        Ok(Self { locals, code })
    }
}

#[derive(Copy, Clone, TryFromPrimitive)]
#[repr(u32)]
enum DataSegmentToken {
    ActiveNoMemIdx = 0,
    Passive = 1,
    ActiveWithMemIdx = 2,
}

impl<A: Allocator + Clone> Parsable<A> for DataSegment<A> {
    fn parse<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<Self, Error<Storage>> {
        let token: DataSegmentToken = parser.read_bounded(context)?;
        match token {
            DataSegmentToken::ActiveNoMemIdx => {
                let offset: Expression<A> = parser.read(context, alloc)?;
                let init: Vec<u8, A> = parser.read(context, alloc)?;
                Ok(Self {
                    init,
                    mode: DataMode::Active(DataModeActive {
                        memory: MemIdx::new(0),
                        offset,
                    }),
                })
            }
            DataSegmentToken::Passive => Ok(Self {
                init: parser.read(context, alloc)?,
                mode: DataMode::Passive(),
            }),
            DataSegmentToken::ActiveWithMemIdx => {
                let memory = parser.read_bounded(context)?;
                let offset: Expression<A> = parser.read(context, alloc)?;
                let init: Vec<u8, A> = parser.read(context, alloc)?;
                Ok(Self {
                    init,
                    mode: DataMode::Active(DataModeActive { memory, offset }),
                })
            }
        }
    }
}
