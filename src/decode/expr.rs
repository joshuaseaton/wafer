// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

//! Logic for re-encoding WebAssembly expressions. (See Expression's docstring
//! for more detail.)

use core::ptr;

use crate::Allocator;
use crate::core_compat;
use crate::core_compat::alloc::collections::TryReserveError;
use crate::core_compat::alloc::{AllocError, Layout};
use crate::core_compat::boxed::Box;
use crate::core_compat::vec::Vec;
use crate::decode::BoundedDecodable;
use crate::storage::Stream;
use crate::types::{
    BlockType, BrTableOperands, BulkOpcode, CallIndirectOperands, Expression, LabelIdx, MemArg,
    Opcode, RefType, SelectTOperands, TableCopyOperands, TableInitOperands, ValType,
};

use super::{ContextStack, Contextual, Decodable, Decoder, Error};

// The maximum natural alignment of any of the structures we use to represent
// instruction operands.
const MAX_NATURAL_ALIGNMENT: usize = 8;

// Allocator wrapper that enables us to ensure that a vector's underlying
// allocation remains `MAX_NATURAL_ALIGNMENT`-aligned at all times.
#[derive(Clone)]
struct AlignedAllocator<A: Allocator>(A);

// Safety: Soundness is deferred to the wrapped allocator.
unsafe impl<A: Allocator> core_compat::alloc::Allocator for AlignedAllocator<A> {
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

// A type that may appear within a decoded Expression, re-encoded by
// 'transcoding' directly from the decoder to the builder.
trait Transcodable<A: Allocator>: Decodable<A> + Contextual {
    fn write_to(self, builder: &mut ExpressionBuilder<A>) -> Result<(), TryReserveError>;

    fn transcode<Storage: Stream>(
        decoder: &mut Decoder<Storage>,
        context: &mut ContextStack,
        builder: &mut ExpressionBuilder<A>,
    ) -> Result<(), Error<Storage>>;
}

impl<T, A> Transcodable<A> for T
where
    T: BoundedDecodable + Contextual,
    A: Allocator,
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
        decoder: &mut Decoder<Storage>,
        context: &mut ContextStack,
        builder: &mut ExpressionBuilder<A>,
    ) -> Result<(), Error<Storage>> {
        let value: Self = decoder.read_bounded(context)?;
        Ok(value.write_to(builder)?)
    }
}

impl<T, A> Transcodable<A> for Vec<T, A>
where
    T: BoundedDecodable + Contextual,
    A: Allocator,
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
        decoder: &mut Decoder<Storage>,
        context: &mut ContextStack,
        builder: &mut ExpressionBuilder<A>,
    ) -> Result<(), Error<Storage>> {
        let len: u32 = decoder.read_bounded(context)?;
        builder.write(len)?;
        for _ in 0..len {
            let elem: T = decoder.read_bounded(context)?;
            builder.write(elem)?;
        }
        Ok(())
    }
}

impl<A: Allocator> Transcodable<A> for BrTableOperands<A> {
    fn write_to(self, builder: &mut ExpressionBuilder<A>) -> Result<(), TryReserveError> {
        self.labels.write_to(builder)?;
        builder.write(self.default)
    }

    fn transcode<Storage: Stream>(
        decoder: &mut Decoder<Storage>,
        context: &mut ContextStack,
        builder: &mut ExpressionBuilder<A>,
    ) -> Result<(), Error<Storage>> {
        Vec::<LabelIdx, A>::transcode(decoder, context, builder)?;
        let default: LabelIdx = decoder.read_bounded(context)?;
        builder.write(default)?;
        Ok(())
    }
}

impl<A: Allocator> Transcodable<A> for SelectTOperands<A> {
    fn write_to(self, builder: &mut ExpressionBuilder<A>) -> Result<(), TryReserveError> {
        builder.write(self.types)
    }

    fn transcode<Storage: Stream>(
        decoder: &mut Decoder<Storage>,
        context: &mut ContextStack,
        builder: &mut ExpressionBuilder<A>,
    ) -> Result<(), Error<Storage>> {
        Vec::<ValType, A>::transcode(decoder, context, builder)
    }
}

// A simple builder for creating
#[derive(Debug)]
struct ExpressionBuilder<A: Allocator> {
    data: Vec<u8, AlignedAllocator<A>>,
}

impl<A: Allocator> ExpressionBuilder<A> {
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

pub(super) fn transcode_expression<A: Allocator, Storage: Stream>(
    decoder: &mut Decoder<Storage>,
    context: &mut ContextStack,
    alloc: &A,
) -> Result<Expression<A>, Error<Storage>> {
    let mut builder = ExpressionBuilder::new(alloc.clone());
    macro_rules! transcode {
        ($operand_type:ty) => {
            <$operand_type>::transcode(decoder, context, &mut builder)
        };
    }
    let mut depth = 0u32;
    loop {
        let op: Opcode = decoder.read_bounded(context)?;
        builder.write(op)?;

        match op {
            Opcode::Block | Opcode::If | Opcode::Loop => {
                transcode!(BlockType)?;
                depth += 1;
            }
            Opcode::Br
            | Opcode::BrIf
            | Opcode::Call
            | Opcode::GlobalGet
            | Opcode::GlobalSet
            | Opcode::LocalGet
            | Opcode::LocalSet
            | Opcode::LocalTee
            | Opcode::RefFunc
            | Opcode::TableGet
            | Opcode::TableSet => transcode!(u32)?,
            Opcode::BrTable => transcode!(BrTableOperands::<A>)?,
            Opcode::BulkPrefix => transcode_bulk_op(decoder, context, &mut builder)?,
            Opcode::CallIndirect => transcode!(CallIndirectOperands)?,
            Opcode::End => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            Opcode::F32Const => transcode!(f32)?,
            Opcode::F32Load
            | Opcode::F32Store
            | Opcode::F64Load
            | Opcode::F64Store
            | Opcode::I32Load
            | Opcode::I32Load8S
            | Opcode::I32Load8U
            | Opcode::I32Load16S
            | Opcode::I32Load16U
            | Opcode::I32Store
            | Opcode::I32Store8
            | Opcode::I32Store16
            | Opcode::I64Load
            | Opcode::I64Load8S
            | Opcode::I64Load8U
            | Opcode::I64Load16S
            | Opcode::I64Load16U
            | Opcode::I64Load32S
            | Opcode::I64Load32U
            | Opcode::I64Store
            | Opcode::I64Store8
            | Opcode::I64Store16
            | Opcode::I64Store32 => transcode!(MemArg)?,
            Opcode::F64Const => transcode!(f64)?,
            Opcode::I32Const => transcode!(i32)?,
            Opcode::I64Const => transcode!(i64)?,
            Opcode::MemoryGrow | Opcode::MemorySize => {
                decoder.read_zero_byte(context)?;
            }
            Opcode::RefNull => transcode!(RefType)?,
            Opcode::SelectT => transcode!(SelectTOperands::<A>)?,
            Opcode::VectorPrefix => transcode_vector_op(decoder, context, &mut builder)?,
            _ => {} // No operands
        }
    }

    Ok(builder.finalize())
}

fn transcode_bulk_op<A: Allocator, Storage: Stream>(
    decoder: &mut Decoder<Storage>,
    context: &mut ContextStack,
    builder: &mut ExpressionBuilder<A>,
) -> Result<(), Error<Storage>> {
    let bulk_op: BulkOpcode = decoder.read_bounded(context)?;
    builder.write(bulk_op)?;

    macro_rules! transcode {
        ($operand_type:ty) => {
            <$operand_type>::transcode(decoder, context, builder)
        };
    }
    match bulk_op {
        BulkOpcode::DataDrop
        | BulkOpcode::ElemDrop
        | BulkOpcode::TableFill
        | BulkOpcode::TableGrow
        | BulkOpcode::TableSize => transcode!(u32)?,
        BulkOpcode::MemoryCopy => {
            decoder.read_zero_byte(context)?;
            decoder.read_zero_byte(context)?;
        }
        BulkOpcode::MemoryFill => {
            decoder.read_zero_byte(context)?;
        }
        BulkOpcode::MemoryInit => {
            transcode!(u32)?;
            decoder.read_zero_byte(context)?;
        }
        BulkOpcode::TableCopy => transcode!(TableCopyOperands)?,
        BulkOpcode::TableInit => transcode!(TableInitOperands)?,
        _ => {} // No operands
    }
    Ok(())
}

fn transcode_vector_op<A: Allocator, Storage: Stream>(
    _decoder: &mut Decoder<Storage>,
    _context: &mut ContextStack,
    _builder: &mut ExpressionBuilder<A>,
) -> Result<(), Error<Storage>> {
    todo!("vector instructions");
}
