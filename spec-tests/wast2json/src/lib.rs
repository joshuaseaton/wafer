// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

//! Common types for WebAssembly specification test format (JSON output from wast2json)

use serde::{Deserialize, Serialize};

/// Top-level structure of a wast2json output file
#[derive(Debug, Deserialize, Serialize)]
pub struct TestFile {
    pub source_filename: String,
    pub commands: Vec<Command>,
}

/// A command in a WAST test file
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Command {
    #[serde(rename = "module")]
    Module(ModuleCommand),
    #[serde(rename = "register")]
    Register(RegisterCommand),
    #[serde(rename = "action")]
    Action(ActionCommand),
    #[serde(rename = "assert_return")]
    AssertReturn(AssertReturnCommand),
    #[serde(rename = "assert_trap")]
    AssertTrap(AssertTrapCommand),
    #[serde(rename = "assert_invalid")]
    AssertInvalid(AssertInvalidCommand),
    #[serde(rename = "assert_malformed")]
    AssertMalformed(AssertMalformedCommand),
    #[serde(rename = "assert_unlinkable")]
    AssertUnlinkable(AssertUnlinkableCommand),
    #[serde(rename = "assert_uninstantiable")]
    AssertUninstantiable(AssertUninstantiableCommand),
    #[serde(rename = "assert_exhaustion")]
    AssertExhaustion(AssertExhaustionCommand),
}

/// Load a WebAssembly module
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModuleCommand {
    pub line: u32,
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Register a module with an alias
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegisterCommand {
    pub line: u32,
    #[serde(rename = "as")]
    pub alias: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Execute an action (standalone action without assertion)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActionCommand {
    pub line: u32,
    pub action: Action,
    pub expected: Vec<Value>,
}

/// Assert that an action returns expected values
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssertReturnCommand {
    pub line: u32,
    pub action: Action,
    pub expected: Vec<Value>,
}

/// Assert that an action traps with a specific message
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssertTrapCommand {
    pub line: u32,
    pub action: Action,
    pub text: Error,
    pub expected: Vec<Value>,
}

/// Assert that a module is invalid
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssertInvalidCommand {
    pub line: u32,
    pub filename: String,
    pub text: Error,
    pub module_type: ModuleType,
}

/// Assert that a module is malformed
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssertMalformedCommand {
    pub line: u32,
    pub filename: String,
    pub text: Error,
    pub module_type: ModuleType,
}

/// Assert that a module fails to link
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssertUnlinkableCommand {
    pub line: u32,
    pub filename: String,
    pub text: Error,
    pub module_type: ModuleType,
}

/// Assert that a module fails to instantiate
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssertUninstantiableCommand {
    pub line: u32,
    pub filename: String,
    pub text: Error,
    pub module_type: ModuleType,
}

/// Assert that an action causes stack exhaustion
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssertExhaustionCommand {
    pub line: u32,
    pub action: Action,
    pub text: Error,
    pub expected: Vec<Value>,
}

/// An action to be performed on a module
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Action {
    #[serde(rename = "invoke")]
    Invoke(InvokeAction),
    #[serde(rename = "get")]
    Get(GetAction),
}

/// Call an exported function
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InvokeAction {
    pub field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    pub args: Vec<Value>,
}

/// Get a global value
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GetAction {
    pub field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
}

/// A WebAssembly value
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Value {
    #[serde(rename = "type")]
    pub value_type: ValueType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// WebAssembly value types
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ValueType {
    I32,
    I64,
    F32,
    F64,
    Externref,
    Funcref,
}

/// Module representation type
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModuleType {
    Text,
    Binary,
}

/// Error types from wast2json (for `assert_malformed`, etc.)
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Error {
    Alignment,
    #[serde(rename = "alignment must not be larger than natural")]
    AlignmentMustNotBeLargerThanNatural,
    #[serde(rename = "call stack exhausted")]
    CallStackExhausted,
    #[serde(rename = "constant expression required")]
    ConstantExpressionRequired,
    #[serde(rename = "constant out of range")]
    ConstantOutOfRange,
    #[serde(rename = "data count and data section have inconsistent lengths")]
    DataCountAndDataSectionHaveInconsistentLengths,
    #[serde(rename = "data count section required")]
    DataCountSectionRequired,
    #[serde(rename = "duplicate export name")]
    DuplicateExportName,
    #[serde(rename = "duplicate func")]
    DuplicateFunc,
    #[serde(rename = "duplicate global")]
    DuplicateGlobal,
    #[serde(rename = "duplicate local")]
    DuplicateLocal,
    #[serde(rename = "duplicate memory")]
    DuplicateMemory,
    #[serde(rename = "duplicate table")]
    DuplicateTable,
    #[serde(rename = "END opcode expected")]
    EndOpcodeExpected,
    #[serde(rename = "function and code section have inconsistent lengths")]
    FunctionAndCodeSectionHaveInconsistentLengths,
    #[serde(rename = "global is immutable")]
    GlobalIsImmutable,
    #[serde(rename = "i32 constant")]
    I32Constant,
    #[serde(rename = "i32 constant out of range")]
    I32ConstantOutOfRange,
    #[serde(rename = "illegal opcode")]
    IllegalOpcode,
    #[serde(rename = "import after function")]
    ImportAfterFunction,
    #[serde(rename = "import after global")]
    ImportAfterGlobal,
    #[serde(rename = "import after memory")]
    ImportAfterMemory,
    #[serde(rename = "import after table")]
    ImportAfterTable,
    #[serde(rename = "incompatible import type")]
    IncompatibleImportType,
    #[serde(rename = "indirect call type mismatch")]
    IndirectCallTypeMismatch,
    #[serde(rename = "inline function type")]
    InlineFunctionType,
    #[serde(rename = "integer divide by zero")]
    IntegerDivideByZero,
    #[serde(rename = "integer overflow")]
    IntegerOverflow,
    #[serde(rename = "integer representation too long")]
    IntegerRepresentationTooLong,
    #[serde(rename = "integer too large")]
    IntegerTooLarge,
    #[serde(rename = "invalid conversion to integer")]
    InvalidConversionToInteger,
    #[serde(rename = "invalid result arity")]
    InvalidResultArity,
    #[serde(rename = "length out of bounds")]
    LengthOutOfBounds,
    #[serde(rename = "magic header not detected")]
    MagicHeaderNotDetected,
    #[serde(rename = "malformed import kind")]
    MalformedImportKind,
    #[serde(rename = "malformed memop flags")]
    MalformedMemopFlags,
    #[serde(rename = "malformed mutability")]
    MalformedMutability,
    #[serde(rename = "malformed reference type")]
    MalformedReferenceType,
    #[serde(rename = "malformed section id")]
    MalformedSectionId,
    #[serde(rename = "malformed UTF-8 encoding")]
    MalformedUtf8Encoding,
    #[serde(rename = "memory size must be at most 65536 pages (4GiB)")]
    MemorySizeMustBeAtMost65536Pages,
    #[serde(rename = "mismatching label")]
    MismatchingLabel,
    #[serde(rename = "multiple memories")]
    MultipleMemories,
    #[serde(rename = "multiple start sections")]
    MultipleStartSections,
    #[serde(rename = "out of bounds memory access")]
    OutOfBoundsMemoryAccess,
    #[serde(rename = "out of bounds table access")]
    OutOfBoundsTableAccess,
    #[serde(rename = "section size mismatch")]
    SectionSizeMismatch,
    #[serde(rename = "size minimum must not be greater than maximum")]
    SizeMinimumMustNotBeGreaterThanMaximum,
    #[serde(rename = "start function")]
    StartFunction,
    #[serde(rename = "too many locals")]
    TooManyLocals,
    #[serde(rename = "type mismatch")]
    TypeMismatch,
    #[serde(rename = "undeclared function reference")]
    UndeclaredFunctionReference,
    #[serde(rename = "undefined element")]
    UndefinedElement,
    #[serde(rename = "unexpected content after last section")]
    UnexpectedContentAfterLastSection,
    #[serde(rename = "unexpected end")]
    UnexpectedEnd,
    #[serde(rename = "unexpected end of section or function")]
    UnexpectedEndOfSectionOrFunction,
    #[serde(rename = "unexpected token")]
    UnexpectedToken,
    #[serde(rename = "uninitialized element")]
    UninitializedElement,
    #[serde(rename = "uninitialized element 2")]
    UninitializedElement2,
    #[serde(rename = "unknown binary version")]
    UnknownBinaryVersion,
    #[serde(rename = "unknown data segment")]
    UnknownDataSegment,
    #[serde(rename = "unknown data segment 1")]
    UnknownDataSegment1,
    #[serde(rename = "unknown elem segment 0")]
    UnknownElemSegment0,
    #[serde(rename = "unknown elem segment 4")]
    UnknownElemSegment4,
    #[serde(rename = "unknown function")]
    UnknownFunction,
    #[serde(rename = "unknown function 7")]
    UnknownFunction7,
    #[serde(rename = "unknown global")]
    UnknownGlobal,
    #[serde(rename = "unknown global 0")]
    UnknownGlobal0,
    #[serde(rename = "unknown global 1")]
    UnknownGlobal1,
    #[serde(rename = "unknown import")]
    UnknownImport,
    #[serde(rename = "unknown label")]
    UnknownLabel,
    #[serde(rename = "unknown local")]
    UnknownLocal,
    #[serde(rename = "unknown memory")]
    UnknownMemory,
    #[serde(rename = "unknown memory 0")]
    UnknownMemory0,
    #[serde(rename = "unknown memory 1")]
    UnknownMemory1,
    #[serde(rename = "unknown operator")]
    UnknownOperator,
    #[serde(rename = "unknown operator anyfunc")]
    UnknownOperatorAnyfunc,
    #[serde(rename = "unknown operator current_memory")]
    UnknownOperatorCurrentMemory,
    #[serde(rename = "unknown operator f32x4.convert_s/i32x4")]
    UnknownOperatorF32x4ConvertSI32x4,
    #[serde(rename = "unknown operator get_global")]
    UnknownOperatorGetGlobal,
    #[serde(rename = "unknown operator get_local")]
    UnknownOperatorGetLocal,
    #[serde(rename = "unknown operator grow_memory")]
    UnknownOperatorGrowMemory,
    #[serde(rename = "unknown operator i32.trunc_s:sat/f32")]
    UnknownOperatorI32TruncSSatF32,
    #[serde(rename = "unknown operator i32.wrap/i64")]
    UnknownOperatorI32WrapI64,
    #[serde(rename = "unknown operator set_global")]
    UnknownOperatorSetGlobal,
    #[serde(rename = "unknown operator set_local")]
    UnknownOperatorSetLocal,
    #[serde(rename = "unknown operator tee_local")]
    UnknownOperatorTeeLocal,
    #[serde(rename = "unknown table")]
    UnknownTable,
    #[serde(rename = "unknown table 0")]
    UnknownTable0,
    #[serde(rename = "unknown type")]
    UnknownType,
    Unreachable,
    #[serde(rename = "zero byte expected")]
    ZeroByteExpected,
}
