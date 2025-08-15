// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

mod expr;
mod validate_impls;

use crate::types::{FuncIdx, FunctionType, ImportDescriptor, Limits, SectionId, TypeIdx};
use crate::{Allocator, Module};

pub(crate) use expr::{ExpressionValidationContext, validate_expression};

/// Represents errors that can arise during module validation.
#[derive(Clone, Copy, Debug)]
pub enum Error {
    DataCountMismatch {
        expected: usize,
        actual: usize,
    },
    DuplicateExportName {
        exportsec_idx: u32,
    },
    FunctionAndCodeSectionMismatch {
        funcsec_size: u32,
        codesec_size: u32,
    },
    IndexOutOfBounds {
        id: SectionId,
        index: u32,
        capacity: u32,
    },
    InvalidMemType(Limits),
    InvalidStartFunction(FuncIdx),
    InvalidTableLimits(Limits),
}

// Called at the end of Module::decode() to reorder the import and export
// sections in a way convenient for validation:
// * imports are *stably* reordered by type, since logical grouping makes for
//   O(1) access by funcidx/tableidx/memidx/globalidx, easier determination of
//   the number of imports by type, and easier separaton later on.
// * exports are reordered by field name, making it easier to determine whether
//   they are all unique.
pub(crate) fn prepare_module_for_validation<A: Allocator>(module: &mut Module<A>) {
    module
        .importsec
        .0
        .sort_by_key(|import| import.descriptor.discriminant());
    module
        .exportsec
        .0
        .sort_by(|a, b| a.field.as_ref().cmp(b.field.as_ref()));
}

pub(crate) struct Validator<'module, A: Allocator> {
    module: &'module Module<A>,

    // The exclusive ending index within the import section of the functions, or
    // `import_tableidx_end` if there are none.
    import_funcidx_end: usize,

    // The exclusive ending index within the import section of the tables, or
    // `import_memidx_end` if there are none.
    import_tableidx_end: usize,

    // The exclusive ending index within the import section of the memories, or
    // the end index of the whole section if there are none.
    import_memidx_end: usize,
}

impl<'module, A: Allocator> Validator<'module, A> {
    fn new(module: &'module Module<A>) -> Self {
        // Recall that the import section was stably sorted by type in
        // prepare_module_for_validation().
        let mut import_tableidx_start = None;
        let mut import_memidx_start = None;
        let mut import_globalidx_start = None;
        for (idx, import) in module.importsec.iter().enumerate() {
            match import.descriptor {
                ImportDescriptor::Function(_) => {}
                ImportDescriptor::Table(_) => {
                    if import_tableidx_start.is_none() {
                        import_tableidx_start = Some(idx);
                    }
                }
                ImportDescriptor::Memory(_) => {
                    if import_memidx_start.is_none() {
                        import_memidx_start = Some(idx);
                    }
                }
                ImportDescriptor::Global(_) => {
                    import_globalidx_start = Some(idx);
                    break;
                }
            }
        }

        let import_memidx_end = import_globalidx_start.unwrap_or(module.importsec.len());
        let import_tableidx_end = import_memidx_start.unwrap_or(import_memidx_end);
        let import_funcidx_end = import_tableidx_start.unwrap_or(import_tableidx_end);
        Self {
            module,
            import_funcidx_end,
            import_tableidx_end,
            import_memidx_end,
        }
    }

    fn data_count(&self) -> usize {
        self.module.datasec.len()
    }

    fn element_count(&self) -> usize {
        self.module.elemsec.len()
    }

    fn function_count(&self) -> usize {
        self.module.funcsec.len() + self.import_funcidx_end
    }

    fn global_count(&self) -> usize {
        self.module.globalsec.len() + (self.module.importsec.len() - self.import_memidx_end)
    }

    fn memory_count(&self) -> usize {
        self.module.memsec.len() + (self.import_memidx_end - self.import_tableidx_end)
    }

    fn table_count(&self) -> usize {
        self.module.tablesec.len() + (self.import_tableidx_end - self.import_funcidx_end)
    }

    fn type_count(&self) -> usize {
        self.module.typesec.len()
    }

    fn function_type(&self, typeidx: TypeIdx) -> &'module FunctionType<A> {
        &self.module.typesec[*typeidx as usize]
    }

    fn function_signature(&self, funcidx: FuncIdx) -> &'module FunctionType<A> {
        let idx = *funcidx as usize;
        let typeidx = if idx < self.import_funcidx_end {
            let import = &self.module.importsec[idx];
            let ImportDescriptor::Function(typeidx) = &import.descriptor else {
                unreachable!();
            };
            *typeidx
        } else {
            let idx = idx - self.import_funcidx_end;
            debug_assert!(idx < self.module.funcsec.len());
            self.module.funcsec[idx]
        };
        self.function_type(typeidx)
    }

    fn validate<T: Validate<A>>(&mut self, value: &T) -> Result<(), Error> {
        value.validate(self)
    }
}

trait Validate<A: Allocator> {
    fn validate(&self, validator: &mut Validator<A>) -> Result<(), Error>;
}

pub(crate) fn validate_module<A: Allocator>(module: &Module<A>) -> Result<(), Error> {
    let mut validator = Validator::new(module);

    // The type section is always valid.
    validator.validate(&module.importsec)?;
    validator.validate(&module.funcsec)?;
    validator.validate(&module.tablesec)?;
    validator.validate(&module.memsec)?;
    validator.validate(&module.globalsec)?;
    validator.validate(&module.exportsec)?;
    if let Some(startsec) = &module.startsec {
        validator.validate(startsec)?;
    }
    validator.validate(&module.elemsec)?;
    validator.validate(&module.codesec)?;
    validator.validate(&module.datasec)?;

    if let Some(count) = module.datacountsec
        && (count as usize) != module.datasec.len()
    {
        return Err(Error::DataCountMismatch {
            expected: count as usize,
            actual: module.datasec.len(),
        });
    }

    Ok(())
}
