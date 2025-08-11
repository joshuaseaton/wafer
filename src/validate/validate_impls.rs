// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

use core::ops::Deref as _;

use crate::Allocator;
use crate::core_compat::vec::Vec;
use crate::types::*;

use super::{Error, ExpressionValidationContext, Validate, Validator, validate_expression};

macro_rules! impl_validate_for_idx {
    ($idx_type:ty, $id:path, $count_method:ident) => {
        impl<A: Allocator> Validate<A> for $idx_type {
            fn validate(&self, validator: &mut Validator<A>) -> Result<(), Error> {
                let index: u32 = **self;
                let capacity = validator.$count_method() as u32;
                if index >= capacity {
                    Err(Error::IndexOutOfBounds {
                        id: $id,
                        index,
                        capacity,
                    })
                } else {
                    Ok(())
                }
            }
        }
    };
}

macro_rules! impl_validate_for_newtype {
    ($type:ident<A>) => {
        impl<A: Allocator> Validate<A> for $type<A> {
            fn validate(&self, validator: &mut Validator<A>) -> Result<(), Error> {
                validator.validate(self.deref())
            }
        }
    };
    ($type:ty) => {
        impl<A: Allocator> Validate<A> for $type {
            fn validate(&self, validator: &mut Validator<A>) -> Result<(), Error> {
                validator.validate(self.deref())
            }
        }
    };
}

impl<T: Validate<A>, A: Allocator> Validate<A> for Vec<T, A> {
    fn validate(&self, validator: &mut Validator<A>) -> Result<(), Error> {
        for elem in self {
            validator.validate(elem)?;
        }
        Ok(())
    }
}

impl_validate_for_idx!(DataIdx, SectionId::Data, data_count);
impl_validate_for_idx!(ElemIdx, SectionId::Element, element_count);
impl_validate_for_idx!(FuncIdx, SectionId::Function, function_count);
impl_validate_for_idx!(GlobalIdx, SectionId::Global, global_count);
impl_validate_for_idx!(MemIdx, SectionId::Memory, memory_count);
impl_validate_for_idx!(TableIdx, SectionId::Table, table_count);
impl_validate_for_idx!(TypeIdx, SectionId::Type, type_count);

impl_validate_for_newtype!(DataSection<A>);
impl_validate_for_newtype!(ElementSection<A>);
impl_validate_for_newtype!(FunctionSection<A>);
impl_validate_for_newtype!(GlobalSection<A>);
impl_validate_for_newtype!(ImportSection<A>);
impl_validate_for_newtype!(MemorySection<A>);
impl_validate_for_newtype!(TableSection<A>);

impl<A: Allocator> Validate<A> for BlockType {
    fn validate(&self, validator: &mut Validator<A>) -> Result<(), Error> {
        if let Self::TypeIndex(idx) = self {
            validator.validate(idx)
        } else {
            Ok(())
        }
    }
}

impl<A: Allocator> Validate<A> for CodeSection<A> {
    fn validate(&self, validator: &mut Validator<A>) -> Result<(), Error> {
        let funcsec = &validator.module.funcsec;
        if funcsec.len() != self.len() {
            return Err(Error::FunctionAndCodeSectionMismatch {
                funcsec_size: funcsec.len() as u32,
                codesec_size: self.len() as u32,
            });
        }

        for (typeidx, function) in funcsec.iter().copied().zip(self.iter()) {
            let func_type = validator.function_type(typeidx);
            validate_expression(
                validator,
                &function.code,
                ExpressionValidationContext::Function(func_type),
            )?;
        }
        Ok(())
    }
}

impl<A: Allocator> Validate<A> for DataSegment<A> {
    fn validate(&self, validator: &mut Validator<A>) -> Result<(), Error> {
        let DataMode::Active(active) = &self.mode else {
            return Ok(());
        };
        validator.validate(&active.memory)?;
        validate_expression(
            validator,
            &active.offset,
            ExpressionValidationContext::Constant(ValType::I32),
        )
    }
}

impl<A: Allocator> Validate<A> for ElementSegment<A> {
    fn validate(&self, validator: &mut Validator<A>) -> Result<(), Error> {
        match &self.init {
            ElementInit::FunctionIndices(funcs) => validator.validate(funcs),
            ElementInit::Expressions(exprs) => {
                for expr in exprs {
                    validate_expression(
                        validator,
                        expr,
                        ExpressionValidationContext::Constant(self.ty.into()),
                    )?;
                }
                Ok(())
            }
        }?;
        if let ElementMode::Active(active) = &self.mode {
            validator.validate(&active.table)?;
            validate_expression(
                validator,
                &active.offset,
                ExpressionValidationContext::Constant(ValType::I32),
            )?;
        }
        Ok(())
    }
}

impl<A: Allocator> Validate<A> for Export<A> {
    fn validate(&self, validator: &mut Validator<A>) -> Result<(), Error> {
        match &self.descriptor {
            ExportDescriptor::Function(funcidx) => validator.validate(funcidx),
            ExportDescriptor::Table(tableidx) => validator.validate(tableidx),
            ExportDescriptor::Memory(memidx) => validator.validate(memidx),
            ExportDescriptor::Global(globalidx) => validator.validate(globalidx),
        }
    }
}

impl<A: Allocator> Validate<A> for ExportSection<A> {
    fn validate(&self, validator: &mut Validator<A>) -> Result<(), Error> {
        // Export names must be distinct. Since we ordered by name in
        // prepare_module_for_validation(), we can just iterate through with
        // pairwise comparison to determine this.
        for idx in 1..self.len() {
            let prev = (*self)[idx - 1].field.as_ref();
            let curr = (*self)[idx].field.as_ref();
            if prev == curr {
                return Err(Error::DuplicateExportName {
                    exportsec_idx: idx as u32,
                });
            }
        }
        validator.validate(&**self)
    }
}

impl<A: Allocator> Validate<A> for Expression<A> {
    fn validate(&self, _validator: &mut Validator<A>) -> Result<(), Error> {
        todo!()
    }
}

impl<A: Allocator> Validate<A> for Global<A> {
    fn validate(&self, validator: &mut Validator<A>) -> Result<(), Error> {
        validate_expression(
            validator,
            &self.init,
            ExpressionValidationContext::Constant(self.ty.value),
        )
    }
}

impl<A: Allocator> Validate<A> for Import<A> {
    fn validate(&self, validator: &mut Validator<A>) -> Result<(), Error> {
        match &self.descriptor {
            ImportDescriptor::Function(typeidx) => validator.validate(typeidx),
            ImportDescriptor::Table(table) => validator.validate(table),
            ImportDescriptor::Memory(mem) => validator.validate(mem),
            ImportDescriptor::Global(_) => Ok(()), // A GlobalType is always valid
        }
    }
}

impl<A: Allocator> Validate<A> for MemType {
    fn validate(&self, _validator: &mut Validator<A>) -> Result<(), Error> {
        const BOUND: u32 = (u16::MAX as u32) + 1;
        let max = self.max.unwrap_or(BOUND);
        if self.min > BOUND || self.min > max || max > BOUND {
            Err(Error::InvalidMemType(**self))
        } else {
            Ok(())
        }
    }
}

impl<A: Allocator> Validate<A> for StartSection {
    fn validate(&self, validator: &mut Validator<A>) -> Result<(), Error> {
        let funcidx = **self;
        validator.validate(&funcidx)?;
        let func = validator.function_signature(funcidx);
        if !func.parameters.is_empty() || !func.results.is_empty() {
            return Err(Error::InvalidStartFunction(funcidx));
        }
        Ok(())
    }
}

impl<A: Allocator> Validate<A> for TableType {
    fn validate(&self, _validator: &mut Validator<A>) -> Result<(), Error> {
        if let Some(max) = self.limits.max
            && self.limits.min > max
        {
            Err(Error::InvalidTableLimits(self.limits))
        } else {
            Ok(())
        }
    }
}
