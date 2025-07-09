// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

//! Logic for re-encoding WebAssembly expressions. (See Expression's docstring
//! for more detail.)

use core::ptr;

use crate::core_compat::alloc::collections::TryReserveError;
use crate::core_compat::alloc::{AllocError, Allocator, Layout};
use crate::core_compat::boxed::Box;
use crate::core_compat::vec::Vec;
use crate::parse::BoundedParsable;
use crate::storage::Stream;
use crate::types::{
    BULK_OPCODE_TO_OPERAND_TYPE, BULK_OPERAND_TYPE_VARIANT_COUNT, OPCODE_TO_OPERAND_TYPE,
    OPERAND_TYPE_VARIANT_COUNT,
};
use crate::types::{
    BlockType, BrTableOperands, BulkOpcode, BulkOperandType, CallIndirectOperands, Expression,
    LabelIdx, MemArg, Opcode, OperandType, RefType, SelectTOperands, TableCopyOperands,
    TableInitOperands, ValType,
};

use super::{ContextStack, Contextual, Error, Parsable, Parser};

// The maximum natural alignment of any of the structures we use to represent
// instruction operands.
const MAX_NATURAL_ALIGNMENT: usize = 8;

// Allocator wrapper that enables us to ensure that a vector's underlying
// allocation remains `MAX_NATURAL_ALIGNMENT`-aligned at all times.
#[derive(Clone)]
struct AlignedAllocator<A: Allocator>(A);

// Safety: Soundness is deferred to the wrapped allocator.
unsafe impl<A: Allocator> Allocator for AlignedAllocator<A> {
    fn allocate(&self, layout: Layout) -> Result<ptr::NonNull<[u8]>, AllocError> {
        let layout = layout.align_to(MAX_NATURAL_ALIGNMENT).unwrap();
        self.0.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: ptr::NonNull<u8>, layout: Layout) {
        let layout = layout.align_to(MAX_NATURAL_ALIGNMENT).unwrap();
        // Safety: Soundness is deferred to the wrapped allocator.
        unsafe { self.0.deallocate(ptr, layout) }
    }

    unsafe fn grow(
        &self,
        ptr: ptr::NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<ptr::NonNull<[u8]>, AllocError> {
        let old_layout = old_layout.align_to(MAX_NATURAL_ALIGNMENT).unwrap();
        let new_layout = new_layout.align_to(MAX_NATURAL_ALIGNMENT).unwrap();
        // Safety: Soundness is deferred to the wrapped allocator.
        unsafe { self.0.grow(ptr, old_layout, new_layout) }
    }

    unsafe fn shrink(
        &self,
        ptr: ptr::NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<ptr::NonNull<[u8]>, AllocError> {
        let old_layout = old_layout.align_to(MAX_NATURAL_ALIGNMENT).unwrap();
        let new_layout = new_layout.align_to(MAX_NATURAL_ALIGNMENT).unwrap();
        // Safety: Soundness is deferred to the wrapped allocator.
        unsafe { self.0.shrink(ptr, old_layout, new_layout) }
    }
}

// A type that may appear within a parsed Expression, re-encoded by
// 'transcoding' directly from the parser to the builder.
trait Transcodable<A: Allocator + Clone>: Parsable<A> + Contextual {
    fn write_to(self, builder: &mut ExpressionBuilder<A>) -> Result<(), TryReserveError>;

    fn transcode<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        builder: &mut ExpressionBuilder<A>,
    ) -> Result<(), Error<Storage>>;
}

impl<T, A> Transcodable<A> for T
where
    T: BoundedParsable + Contextual,
    A: Allocator + Clone,
{
    fn write_to(self, builder: &mut ExpressionBuilder<A>) -> Result<(), TryReserveError> {
        let data = &mut builder.data;

        // The alignment of `data`'s allocation ensures that the byte at
        // `aligned_pos` has T's natural alignment.
        let pos = data.len();
        let aligned_pos = pos.next_multiple_of(align_of::<Self>());
        let padding = aligned_pos - pos;
        data.try_reserve_exact(padding + size_of::<Self>())?;
        if padding > 0 {
            data.resize(pos + padding, 0);
        }

        // Safety: Per the above, the address being written to has T's natural
        // alignment, and the call to try_reserve_exact() ensures that the
        // capacity is `aligned_pos + size_of::<Self>()`.
        unsafe {
            let write_ptr = data.as_mut_ptr().add(aligned_pos);
            write_ptr.cast::<Self>().write(self);
            data.set_len(aligned_pos + size_of::<Self>());
        }
        Ok(())
    }

    fn transcode<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        builder: &mut ExpressionBuilder<A>,
    ) -> Result<(), Error<Storage>> {
        let value: Self = parser.read_bounded(context)?;
        Ok(value.write_to(builder)?)
    }
}

impl<T, A> Transcodable<A> for Vec<T, A>
where
    T: BoundedParsable + Contextual,
    A: Allocator + Clone,
    Vec<T, A>: Contextual,
{
    fn write_to(self, builder: &mut ExpressionBuilder<A>) -> Result<(), TryReserveError> {
        builder.write(self.len() as u32)?;
        for elem in &self {
            builder.write(*elem)?;
        }
        Ok(())
    }

    fn transcode<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        builder: &mut ExpressionBuilder<A>,
    ) -> Result<(), Error<Storage>> {
        let len: u32 = parser.read_bounded(context)?;
        builder.write(len)?;
        for _ in 0..len {
            let elem: T = parser.read_bounded(context)?;
            builder.write(elem)?;
        }
        Ok(())
    }
}

impl<A: Allocator + Clone> Transcodable<A> for BrTableOperands<A> {
    fn write_to(self, builder: &mut ExpressionBuilder<A>) -> Result<(), TryReserveError> {
        self.labels.write_to(builder)?;
        builder.write(self.default)
    }

    fn transcode<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        builder: &mut ExpressionBuilder<A>,
    ) -> Result<(), Error<Storage>> {
        Vec::<LabelIdx, A>::transcode(parser, context, builder)?;
        let default: LabelIdx = parser.read_bounded(context)?;
        builder.write(default)?;
        Ok(())
    }
}

impl<A: Allocator + Clone> Transcodable<A> for SelectTOperands<A> {
    fn write_to(self, builder: &mut ExpressionBuilder<A>) -> Result<(), TryReserveError> {
        builder.write(self.types)
    }

    fn transcode<Storage: Stream>(
        parser: &mut Parser<Storage>,
        context: &mut ContextStack,
        builder: &mut ExpressionBuilder<A>,
    ) -> Result<(), Error<Storage>> {
        Vec::<ValType, A>::transcode(parser, context, builder)
    }
}

// A simple builder for creating
#[derive(Debug)]
struct ExpressionBuilder<A: Allocator + Clone> {
    data: Vec<u8, AlignedAllocator<A>>,
}

impl<A: Allocator + Clone> ExpressionBuilder<A> {
    fn new(alloc: A) -> Self {
        let aligned_alloc = AlignedAllocator(alloc);
        Self {
            data: Vec::new_in(aligned_alloc),
        }
    }

    fn finalize(self) -> Expression<A> {
        let (ptr, len, _, alloc) = self.data.into_raw_parts_with_alloc();
        let ptr: *mut [u8] = ptr::slice_from_raw_parts_mut(ptr, len);
        // Safety: The allocation is truly being managed by the wrapped
        // allocator A.
        unsafe { Expression::new(Box::from_raw_in(ptr, alloc.0)) }
    }

    fn write<T: Transcodable<A>>(&mut self, value: T) -> Result<(), TryReserveError> {
        value.write_to(self)
    }
}

// Type alias for function pointer table entries
type TranscoderFn<A, Storage> = fn(
    &mut Parser<Storage>,
    &mut ContextStack,
    &mut ExpressionBuilder<A>,
) -> Result<(), Error<Storage>>;

pub(super) fn transcode_expression<A: Allocator + Clone, Storage: Stream>(
    parser: &mut Parser<Storage>,
    context: &mut ContextStack,
    alloc: &A,
) -> Result<Expression<A>, Error<Storage>> {
    // The strategy here is to efficiently reduce operand transcoding through
    // two loads two loads and an indirect call: the operand type is looked up
    // in OPCODE_TO_OPERAND_TYPE, which then gives as an index to into the small
    // transcoder table constructed below.

    let operand_transcoders = {
        let mut operand_transcoders: [TranscoderFn<A, Storage>; OPERAND_TYPE_VARIANT_COUNT] =
            [|_, _, _| unreachable!(); OPERAND_TYPE_VARIANT_COUNT];

        macro_rules! set {
            ($operand_type:path, $type:ty) => {
                operand_transcoders[$operand_type as usize] = <$type as Transcodable<A>>::transcode;
            };
        }
        set!(OperandType::BlockType, BlockType);
        set!(OperandType::BrTable, BrTableOperands<A>);
        set!(OperandType::CallIndirect, CallIndirectOperands);
        set!(OperandType::F32, f32);
        set!(OperandType::F64, f64);
        set!(OperandType::I32, i32);
        set!(OperandType::I64, i64);
        set!(OperandType::MemArg, MemArg);
        set!(OperandType::RefType, RefType);
        set!(OperandType::SelectT, SelectTOperands<A>);
        set!(OperandType::U32, u32);
        operand_transcoders[OperandType::None as usize] = |_, _, _| Ok(());
        operand_transcoders[OperandType::BulkOp as usize] = transcode_bulk_op;
        operand_transcoders[OperandType::VectorOp as usize] = transcode_vector_op;
        operand_transcoders
    };

    let mut builder = ExpressionBuilder::new(alloc.clone());
    let mut depth = 0u32;
    loop {
        let op: Opcode = parser.read_bounded(context)?;
        builder.write(op)?;

        let operand_type = OPCODE_TO_OPERAND_TYPE[op as usize];
        let transcode_operands = operand_transcoders[operand_type as usize];
        transcode_operands(parser, context, &mut builder)?;

        match op {
            Opcode::End => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            Opcode::Block | Opcode::Loop | Opcode::If => {
                depth += 1;
            }
            Opcode::MemoryGrow | Opcode::MemorySize => {
                parser.read_zero_byte(context)?;
            }
            _ => {}
        }
    }

    Ok(builder.finalize())
}

fn transcode_bulk_op<A: Allocator + Clone, Storage: Stream>(
    parser: &mut Parser<Storage>,
    context: &mut ContextStack,
    builder: &mut ExpressionBuilder<A>,
) -> Result<(), Error<Storage>> {
    // The strategy here is to efficiently reduce bulk operand transcoding
    // through two loads two loads and an indirect call: the bulk operand type
    // is looked up in BULK_OPCODE_TO_OPERAND_TYPE, which then gives as an index
    // to into the small transcoder table constructed below.

    let operand_transcoders = {
        let mut operand_transcoders: [TranscoderFn<A, Storage>; BULK_OPERAND_TYPE_VARIANT_COUNT] =
            [|_, _, _| unreachable!(); BULK_OPERAND_TYPE_VARIANT_COUNT];
        macro_rules! set {
            ($operand_type:path, $type:ty) => {
                operand_transcoders[$operand_type as usize] = <$type as Transcodable<A>>::transcode;
            };
        }
        set!(BulkOperandType::TableCopyOperands, TableCopyOperands);
        set!(BulkOperandType::TableInitOperands, TableInitOperands);
        set!(BulkOperandType::U32, u32);
        operand_transcoders[OperandType::None as usize] = |_, _, _| Ok(());
        operand_transcoders
    };

    let bulk_op: BulkOpcode = parser.read_bounded(context)?;
    builder.write(bulk_op)?;

    let operand_type = BULK_OPCODE_TO_OPERAND_TYPE[bulk_op as usize];
    let transcode_operands = operand_transcoders[operand_type as usize];
    transcode_operands(parser, context, builder)?;

    // Handle special reserved bytes for memory operations
    match bulk_op {
        BulkOpcode::MemoryInit | BulkOpcode::MemoryFill => parser.read_zero_byte(context)?,
        BulkOpcode::MemoryCopy => {
            parser.read_zero_byte(context)?;
            parser.read_zero_byte(context)?;
        }
        _ => {}
    }
    Ok(())
}

fn transcode_vector_op<A: Allocator + Clone, Storage: Stream>(
    _parser: &mut Parser<Storage>,
    _context: &mut ContextStack,
    _builder: &mut ExpressionBuilder<A>,
) -> Result<(), Error<Storage>> {
    todo!("vector instructions");
}
