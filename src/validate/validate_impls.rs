// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

use core::ops::Deref as _;

use crate::Allocator;
use crate::core_compat::vec::Vec;
use crate::types::*;

use super::{Error, Validate, ValidationContext, validate_constant_expression};

macro_rules! impl_validate_for_idx {
    ($idx_type:ty, $check_fn:ident) => {
        impl Validate for $idx_type {
            fn validate(&self, context: &mut ValidationContext) -> Result<(), Error> {
                context.$check_fn(*self)
            }
        }
    };
}

macro_rules! impl_validate_for_newtype {
    ($newtype:ident <A>) => {
        impl<A: Allocator> Validate for $newtype<A> {
            fn validate(&self, context: &mut ValidationContext) -> Result<(), Error> {
                self.deref().validate(context)
            }
        }
    };
}

impl<T> Validate for Option<T>
where
    T: Validate,
{
    fn validate(&self, context: &mut ValidationContext) -> Result<(), Error> {
        match self {
            Some(item) => item.validate(context),
            None => Ok(()),
        }
    }
}

impl<T, A: Allocator> Validate for Vec<T, A>
where
    T: Validate,
{
    fn validate(&self, context: &mut ValidationContext) -> Result<(), Error> {
        for item in self {
            item.validate(context)?;
        }
        Ok(())
    }
}

impl_validate_for_idx!(DataIdx, check_dataidx);
impl_validate_for_idx!(ElemIdx, check_elemidx);
impl_validate_for_idx!(FuncIdx, check_funcidx);
impl_validate_for_idx!(GlobalIdx, check_globalidx);
impl_validate_for_idx!(MemIdx, check_memidx);
impl_validate_for_idx!(TableIdx, check_tableidx);
impl_validate_for_idx!(TypeIdx, check_typeidx);

impl_validate_for_newtype!(DataSection<A>);
impl_validate_for_newtype!(ElementSection<A>);
impl_validate_for_newtype!(FunctionSection<A>);
impl_validate_for_newtype!(GlobalSection<A>);
impl_validate_for_newtype!(ImportSection<A>);
impl_validate_for_newtype!(MemorySection<A>);
impl_validate_for_newtype!(TableSection<A>);

impl Validate for BlockType {
    fn validate(&self, context: &mut ValidationContext) -> Result<(), Error> {
        if let BlockType::TypeIndex(idx) = self {
            idx.validate(context)
        } else {
            Ok(())
        }
    }
}

impl<A: Allocator> Validate for DataSegment<A> {
    fn validate(&self, context: &mut ValidationContext) -> Result<(), Error> {
        let DataMode::Active(active) = &self.mode else {
            return Ok(());
        };
        active.memory.validate(context)?;
        validate_constant_expression(context, &active.offset, ValType::I32)
    }
}

impl<A: Allocator> Validate for ElementSegment<A> {
    fn validate(&self, context: &mut ValidationContext) -> Result<(), Error> {
        match &self.init {
            ElementInit::FunctionIndices(funcs) => {
                funcs.validate(context)?;
            }
            ElementInit::Expressions(exprs) => {
                for expr in exprs {
                    validate_constant_expression(context, expr, self.ty.into())?;
                }
            }
        }
        if let ElementMode::Active(active) = &self.mode {
            active.table.validate(context)?;
            validate_constant_expression(context, &active.offset, ValType::I32)?;
        }
        Ok(())
    }
}

impl<A: Allocator> Validate for Export<A> {
    fn validate(&self, context: &mut ValidationContext) -> Result<(), Error> {
        match &self.descriptor {
            ExportDescriptor::Function(funcidx) => funcidx.validate(context),
            ExportDescriptor::Table(tableidx) => tableidx.validate(context),
            ExportDescriptor::Memory(memidx) => memidx.validate(context),
            ExportDescriptor::Global(globalidx) => globalidx.validate(context),
        }
    }
}

impl<A: Allocator> Validate for ExportSection<A> {
    fn validate(&self, context: &mut ValidationContext) -> Result<(), Error> {
        // Export names must be distinct. Since we ordered by name in
        // prepare_module_for_validation(), we can just iterate through with
        // pairwise comparison to determine this.
        for idx in 1..self.len() {
            let prev = self[idx - 1].field.as_ref();
            let curr = self[idx].field.as_ref();
            if prev == curr {
                return Err(Error::DuplicateExportName {
                    exportsec_idx: idx as u32,
                });
            }
        }
        self.deref().validate(context)?;
        Ok(())
    }
}

impl<A: Allocator> Validate for Global<A> {
    fn validate(&self, context: &mut ValidationContext) -> Result<(), Error> {
        validate_constant_expression(context, &self.init, self.ty.value)
    }
}

impl<A: Allocator> Validate for Import<A> {
    fn validate(&self, context: &mut ValidationContext) -> Result<(), Error> {
        match &self.descriptor {
            ImportDescriptor::Function(typeidx) => typeidx.validate(context),
            ImportDescriptor::Table(table) => table.validate(context),
            ImportDescriptor::Memory(mem) => mem.validate(context),
            ImportDescriptor::Global(_) => Ok(()), // A GlobalType is always valid
        }
    }
}

impl Validate for MemType {
    fn validate(&self, _context: &mut ValidationContext) -> Result<(), Error> {
        const BOUND: u32 = (u16::MAX as u32) + 1;

        let max = self.max.unwrap_or(BOUND);
        if self.min > BOUND || self.min > max || max > BOUND {
            Err(Error::InvalidMemType(**self))
        } else {
            Ok(())
        }
    }
}

impl Validate for StartSection {
    fn validate(&self, context: &mut ValidationContext) -> Result<(), Error> {
        let funcidx = **self;
        funcidx.validate(context)?;
        // Note: Function signature validation requires Module context,
        // so this will need to be handled elsewhere or the trait needs access to Module
        Ok(())
    }
}

impl Validate for TableType {
    fn validate(&self, _context: &mut ValidationContext) -> Result<(), Error> {
        if let Some(max) = self.limits.max
            && self.limits.min > max
        {
            Err(Error::InvalidTableLimits(self.limits))
        } else {
            Ok(())
        }
    }
}
