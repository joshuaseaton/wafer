// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

mod expr;
mod validate_impls;

use crate::types::{
    DataIdx, ElemIdx, FuncIdx, FunctionType, GlobalIdx, ImportDescriptor, Limits, MemIdx,
    SectionId, TableIdx, TypeIdx,
};
use crate::{Allocator, Module};

use expr::{validate_constant_expression, validate_function};

/// Represents errors that can arise during module validation.
#[derive(Clone, Copy, Debug)]
pub enum Error {
    DataCountMismatch {
        expected: u32,
        actual: u32,
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

//
//
// TODO: Remove allow() once there are other fields.
#[allow(clippy::struct_field_names)]
struct ValidationContext {
    // Holds section counts so that code can be mutated during its validation,
    // which requires a mutable borrow of Module, without needing to further
    // borrow Module (which can't happen).
    data_count: u32,
    element_count: u32,
    function_count: u32,
    global_count: u32,
    memory_count: u32,
    table_count: u32,
    type_count: u32,
}

impl ValidationContext {
    fn new<A: Allocator>(module: &Module<A>) -> Self {
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

        let import_function_count = import_funcidx_end;
        let import_global_count = module.importsec.len() - import_memidx_end;
        let import_memory_count = import_memidx_end - import_tableidx_end;
        let import_table_count = import_tableidx_end - import_funcidx_end;
        Self {
            data_count: module.datasec.len() as u32,
            element_count: module.elemsec.len() as u32,
            function_count: (module.funcsec.len() + import_function_count) as u32,
            global_count: (module.globalsec.len() + import_global_count) as u32,
            memory_count: (module.memsec.len() + import_memory_count) as u32,
            table_count: (module.tablesec.len() + import_table_count) as u32,
            type_count: module.typesec.len() as u32,
        }
    }

    fn check_dataidx(&self, idx: DataIdx) -> Result<(), Error> {
        let index = *idx;
        if index >= self.data_count {
            Err(Error::IndexOutOfBounds {
                id: SectionId::Data,
                index,
                capacity: self.data_count,
            })
        } else {
            Ok(())
        }
    }

    fn check_elemidx(&self, idx: ElemIdx) -> Result<(), Error> {
        let index = *idx;
        if index >= self.element_count {
            Err(Error::IndexOutOfBounds {
                id: SectionId::Element,
                index,
                capacity: self.element_count,
            })
        } else {
            Ok(())
        }
    }

    fn check_funcidx(&self, idx: FuncIdx) -> Result<(), Error> {
        let index = *idx;
        if index >= self.function_count {
            Err(Error::IndexOutOfBounds {
                id: SectionId::Function,
                index,
                capacity: self.function_count,
            })
        } else {
            Ok(())
        }
    }

    fn check_globalidx(&self, idx: GlobalIdx) -> Result<(), Error> {
        let index = *idx;
        if index >= self.global_count {
            Err(Error::IndexOutOfBounds {
                id: SectionId::Global,
                index,
                capacity: self.global_count,
            })
        } else {
            Ok(())
        }
    }

    fn check_memidx(&self, idx: MemIdx) -> Result<(), Error> {
        let index = *idx;
        if index >= self.memory_count {
            Err(Error::IndexOutOfBounds {
                id: SectionId::Memory,
                index,
                capacity: self.memory_count,
            })
        } else {
            Ok(())
        }
    }

    fn check_tableidx(&self, idx: TableIdx) -> Result<(), Error> {
        let index = *idx;
        if index >= self.table_count {
            Err(Error::IndexOutOfBounds {
                id: SectionId::Table,
                index,
                capacity: self.table_count,
            })
        } else {
            Ok(())
        }
    }

    fn check_typeidx(&self, idx: TypeIdx) -> Result<(), Error> {
        let index = *idx;
        if index >= self.type_count {
            Err(Error::IndexOutOfBounds {
                id: SectionId::Type,
                index,
                capacity: self.type_count,
            })
        } else {
            Ok(())
        }
    }
}

impl<A: Allocator> Module<A> {
    fn function_signature(
        &self,
        context: &ValidationContext,
        funcidx: FuncIdx,
    ) -> &FunctionType<A> {
        let idx = *funcidx as usize;
        let num_imports = (context.function_count as usize) - self.funcsec.len();
        let typeidx = if idx < num_imports {
            let import = &self.importsec[idx];
            let ImportDescriptor::Function(typeidx) = &import.descriptor else {
                unreachable!();
            };
            *typeidx
        } else {
            let idx = idx - num_imports;
            debug_assert!(idx < self.funcsec.len());
            self.funcsec[idx]
        };
        &self.typesec[*typeidx as usize]
    }
}

// Validates a substructure of a module.
trait Validate {
    fn validate(&self, context: &mut ValidationContext) -> Result<(), Error>;
}

pub(super) fn validate_module<A: Allocator>(module: &mut Module<A>) -> Result<(), Error> {
    // Stably sort by type, since logical grouping makes for O(1) determination
    // of the number of imports by type, and for easier separaton later on. It
    // is important that we sort stably since the ordering within a group is
    // meaningful to the bytecode.
    module
        .importsec
        .0
        .sort_by_key(|import| import.descriptor.discriminant());

    // Sort exports by field name, making it easier to determine whether they
    // are all unique during ExportSection validation.
    module
        .exportsec
        .0
        .sort_by(|a, b| a.field.as_ref().cmp(b.field.as_ref()));

    // Structural validation of the module.
    //
    // Note that the type section is always valid.
    let mut context = ValidationContext::new(module);
    module.importsec.validate(&mut context)?;
    module.funcsec.validate(&mut context)?;
    module.tablesec.validate(&mut context)?;
    module.memsec.validate(&mut context)?;
    module.globalsec.validate(&mut context)?;
    module.exportsec.validate(&mut context)?;
    module.elemsec.validate(&mut context)?;
    module.datasec.validate(&mut context)?;

    if let Some(start_idx) = module.startsec {
        start_idx.validate(&mut context)?;
        let func = module.function_signature(&context, *start_idx);
        if !func.parameters.is_empty() || !func.results.is_empty() {
            return Err(Error::InvalidStartFunction(*start_idx));
        }
    }

    if module.funcsec.len() != module.codesec.len() {
        return Err(Error::FunctionAndCodeSectionMismatch {
            funcsec_size: module.funcsec.len() as u32,
            codesec_size: module.codesec.len() as u32,
        });
    }

    if let Some(count) = module.datacountsec
        && count != (module.datasec.len() as u32)
    {
        return Err(Error::DataCountMismatch {
            expected: count,
            actual: module.datasec.len() as u32,
        });
    }

    // Now it's time for code validation and patching. The function and code
    // sections were validated as having the same length above.
    for (typeidx, function) in module
        .funcsec
        .iter()
        .copied()
        .zip(module.codesec.iter_mut())
    {
        let func_type = &module.typesec[*typeidx as usize];
        validate_function(&mut context, &mut function.code, func_type)?;
    }
    Ok(())
}
