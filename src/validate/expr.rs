// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

use crate::Allocator;
use crate::types::{Expression, FunctionType, ValType};

use super::{Error, ValidationContext};

// TODO: Implement me.
#[allow(clippy::unnecessary_wraps, unused)]
pub(crate) fn validate_function<A: Allocator>(
    context: &mut ValidationContext,
    expr: &mut Expression<A>,
    typ: &FunctionType<A>,
) -> Result<(), Error> {
    Ok(())
}

// TODO: Implement me.
#[allow(clippy::unnecessary_wraps, unused)]
pub(crate) fn validate_constant_expression<A: Allocator>(
    context: &mut ValidationContext,
    expr: &Expression<A>,
    typ: ValType,
) -> Result<(), Error> {
    Ok(())
}
