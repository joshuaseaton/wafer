// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

use crate::Allocator;
use crate::types::{Expression, FunctionType, ValType};

use super::{Error, Validator};

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
pub(crate) enum ExpressionValidationContext<'module, A: Allocator> {
    Function(&'module FunctionType<A>),
    Constant(ValType),
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps, unused)]
pub(crate) fn validate_expression<A: Allocator>(
    validator: &mut Validator<A>,
    expr: &Expression<A>,
    context: ExpressionValidationContext<A>,
) -> Result<(), Error> {
    // TODO: implement me.
    Ok(())
}
