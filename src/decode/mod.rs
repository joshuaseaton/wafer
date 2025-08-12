// Copyright (c) 2025 Joshua Seaton
//
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT

//! WebAssembly binary format parsing.

mod decodable_impls;
mod expr;
mod leb128;

use expr::transcode_expression;

use core::fmt;

use num_enum::TryFromPrimitive;

use leb128::Leb128;

use crate::core_compat::alloc::collections::TryReserveError;
use crate::core_compat::boxed::Box;
use crate::core_compat::vec::Vec;
use crate::storage::Stream;
use crate::types::{
    CodeSection, CustomSection, DataSection, ElementSection, ExportSection, FunctionSection,
    GlobalSection, ImportSection, MemorySection, Name, SectionId, TableSection, TypeSection,
    Version,
};
use crate::{Allocator, Module};

// The maximum parsing depth of this implementation (which is also pretty much
// the lower bound implicitly suggested by the spec).
const MAX_DEPTH: usize = 6;

// We represent this as an enum with one value to leverage existing "decode this
// u32 enum" machinery to check for a valid magic value.
#[derive(Clone, Copy, Debug, TryFromPrimitive)]
#[repr(u32)]
enum Magic {
    Value = 0x6d_73_61_00, // '\0asm'
}

// Represents parsing context.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
enum ContextId {
    #[default]
    Invalid,
    BlockType,
    BrTableOperands,
    BulkOpcode,
    Byte,
    CodeSec,
    CustomSec,
    Data,
    DataIdx,
    DataSec,
    DataToken,
    Elem,
    ElemIdx,
    ElemKind,
    ElemSec,
    ElemToken,
    Export,
    ExportDesc,
    ExportDescToken,
    ExportSec,
    Expr,
    F32,
    F64,
    Func,
    FuncIdx,
    FuncType,
    FuncTypeToken,
    FuncSec,
    Global,
    GlobalIdx,
    GlobalSec,
    GlobalType,
    I32,
    I64,
    Import,
    ImportDesc,
    ImportDescToken,
    ImportSec,
    LabelIdx,
    Limits,
    LimitsMaxToken,
    LocalIdx,
    Locals,
    Magic,
    MemArg,
    MemIdx,
    MemType,
    MemorySec,
    Mut,
    Name,
    Opcode,
    ReadingBytes,
    RefType,
    ResultType,
    SectionId,
    SelectTOperands,
    SkippingBytes,
    StartSec,
    TableIdx,
    TableSec,
    TableType,
    TypeIdx,
    TypeSec,
    U32,
    ValType,
    VecByte,
    VecCode,
    VecExpr,
    VecFuncIdx,
    VecLabelIdx,
    VecValType,
    Version,
}

impl From<ContextId> for &'static str {
    fn from(id: ContextId) -> Self {
        match id {
            ContextId::Invalid => unreachable!("invalid context somehow reached!?"),
            ContextId::BrTableOperands => "br_table operands",
            ContextId::BulkOpcode => "bulk opcode",
            ContextId::Byte => "byte",
            ContextId::CodeSec => "codesec",
            ContextId::CustomSec => "customsec",
            ContextId::Data => "data",
            ContextId::DataIdx => "dataidx",
            ContextId::DataSec => "datasec",
            ContextId::DataToken => "data token",
            ContextId::Elem => "elem",
            ContextId::ElemIdx => "elemidx",
            ContextId::ElemKind => "elemkind",
            ContextId::ElemSec => "elemsec",
            ContextId::ElemToken => "elem token",
            ContextId::Func => "func",
            ContextId::Export => "export",
            ContextId::ExportDesc => "exportdesc",
            ContextId::ExportDescToken => "exportdesc token",
            ContextId::ExportSec => "exportsec",
            ContextId::Expr => "expr",
            ContextId::F32 => "f32",
            ContextId::F64 => "f64",
            ContextId::FuncIdx => "funcidx",
            ContextId::FuncType => "functype",
            ContextId::FuncTypeToken => "functype token",
            ContextId::FuncSec => "funcsec",
            ContextId::Global => "global",
            ContextId::GlobalIdx => "globalidx",
            ContextId::GlobalSec => "globalsec",
            ContextId::GlobalType => "globaltype",
            ContextId::I32 => "i32",
            ContextId::I64 => "i64",
            ContextId::Import => "import",
            ContextId::ImportDesc => "importdesc",
            ContextId::ImportDescToken => "importdesc token",
            ContextId::ImportSec => "importsec",
            ContextId::LabelIdx => "labelidx",
            ContextId::Limits => "limits",
            ContextId::LimitsMaxToken => "limits max token",
            ContextId::LocalIdx => "localidx",
            ContextId::Locals => "locals",
            ContextId::Magic => "magic",
            ContextId::MemArg => "memarg",
            ContextId::MemIdx => "memidx",
            ContextId::MemType => "memtype",
            ContextId::MemorySec => "memsec",
            ContextId::Mut => "mut",
            ContextId::Name => "name",
            ContextId::Opcode => "opcode",
            ContextId::ReadingBytes => "reading bytes",
            ContextId::RefType => "reftype",
            ContextId::ResultType => "resulttype",
            ContextId::SectionId => "section ID",
            ContextId::SelectTOperands => "select_t operands",
            ContextId::SkippingBytes => "skipping bytes",
            ContextId::StartSec => "startsec",
            ContextId::TableIdx => "tableidx",
            ContextId::TableSec => "tablesec",
            ContextId::TableType => "tabletype",
            ContextId::TypeIdx => "typeidx",
            ContextId::TypeSec => "typesec",
            ContextId::U32 => "u32",
            ContextId::ValType => "valtype",
            ContextId::VecByte => "vec(byte)",
            ContextId::BlockType => "blocktype",
            ContextId::VecCode => "vec(code)",
            ContextId::VecExpr => "vec(expr)",
            ContextId::VecFuncIdx => "vec(funcidx)",
            ContextId::VecLabelIdx => "vec(labelidx)",
            ContextId::VecValType => "vec(valtype)",
            ContextId::Version => "version",
        }
    }
}

trait Contextual {
    const ID: ContextId;
}

// A frame of parsing context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ContextFrame {
    // A description of what is being parsed.
    context: &'static str,

    // Byte offset in the stream where this context was entered.
    offset: usize,
}

/// Stack for tracking parsing context during error reporting.
#[derive(Clone, Debug, Default)]
pub(crate) struct ContextStack {
    offsets: [usize; MAX_DEPTH],
    ids: [ContextId; MAX_DEPTH],
    depth: u8,
}

impl ContextStack {
    // Pushes a new context frame, returning true if successful.
    fn push(&mut self, id: ContextId, offset: usize) -> bool {
        let depth = self.depth as usize;
        if depth >= MAX_DEPTH {
            return false;
        }
        self.offsets[depth] = offset;
        self.ids[depth] = id;
        self.depth += 1;
        true
    }

    // Pop the top context frame.
    fn pop(&mut self) {
        debug_assert!(self.depth > 0, "{self:#?}");
        self.depth -= 1;
    }

    // Returns an iterator over frames in "pushed" order (outermost to
    // innermost).
    fn iter(&self) -> impl Iterator<Item = ContextFrame> + '_ {
        self.offsets
            .iter()
            .zip(&self.ids)
            .take(self.depth as usize)
            .map(|(&offset, &id)| ContextFrame {
                context: id.into(),
                offset,
            })
    }
}

/// A parsing error with additional context around what hierarchy of things were
/// being decoded at the time.
pub struct ErrorWithContext<Storage: Stream> {
    /// The underlying parsing error.
    pub error: Error<Storage>,
    pub(crate) context: ContextStack,
}

impl<Storage: Stream> fmt::Debug for ErrorWithContext<Storage> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.error)?;
        for (i, frame) in self.context.iter().enumerate() {
            write!(f, "\n{:#x}: ", frame.offset)?;
            for _ in 0..i {
                write!(f, "  ")?;
            }
            write!(f, "{}", frame.context)?;
        }
        Ok(())
    }
}

/// Represents errors that can arise during module parsing.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Error<Storage: Stream> {
    /// Failed memory allocation.
    AllocError,
    /// A given section appears more than once in the module.
    DuplicateSection(SectionId),
    /// Decoder context stack exceeded maximum depth to prevent stack overflow.
    ExcessiveParsingDepth {
        context: &'static str,
        offset: usize,
    },
    /// Invalid bulk memory/table operation opcode encountered.
    InvalidBulkOpcode(u32),
    /// Invalid data segment token encountered.
    InvalidDataToken(u32),
    /// Invalid element segment token encountered.
    InvalidElementToken(u32),
    /// Function body length doesn't match the declared length.
    InvalidFunctionLength { expected: u32, actual: u32 },
    /// Invalid LEB128 encoding encountered.
    InvalidLeb128,
    /// Invalid WebAssembly magic number.
    InvalidMagic(u32),
    /// Section length doesn't match the declared length.
    InvalidSectionLength {
        id: SectionId,
        expected: u32,
        actual: u32,
    },
    /// Invalid byte token encountered during parsing.
    InvalidToken(u8),
    /// Invalid UTF-8 encoding in a name field.
    InvalidUtf8,
    /// Invalid value type encoding encountered.
    InvalidValType(u8),
    /// (Non-custom) sections appear in the wrong order.
    OutOfOrderSection { before: SectionId, after: SectionId },
    /// Error from the underlying storage.
    Storage(Storage::Error),
    /// Function declares too many local variables (exceeding an
    /// implementation-defined limit).
    TooManyLocals(usize),
    /// Unsupported WebAssembly version number.
    UnknownVersion(u32),
}

impl<Storage: Stream> fmt::Debug for Error<Storage> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::AllocError => write!(f, "allocation failure"),
            Error::DuplicateSection(id) => write!(f, "duplicate of section ({id:?})"),
            Error::ExcessiveParsingDepth { context, offset } => {
                write!(f, "unexpected frame at {offset:#x}: {context}")
            }
            Error::InvalidBulkOpcode(op) => write!(f, "invalid bulk opcode ({op:#x})"),
            Error::InvalidDataToken(token) => write!(f, "invalid data token ({token:#x})"),
            Error::InvalidElementToken(token) => write!(f, "invalid element token ({token:#x})"),
            Error::InvalidFunctionLength { expected, actual } => write!(
                f,
                "invalid func length: expected {expected:#x}; got {actual:#x}"
            ),
            Error::InvalidLeb128 => write!(f, "invalid LEB128-encoding"),
            Error::InvalidMagic(magic) => write!(f, "invalid magic ({magic:#x})"),
            Error::InvalidSectionLength {
                id,
                expected,
                actual,
            } => write!(
                f,
                "invalid section length for {id:?}: expected {expected:#x}; got {actual:#x}"
            ),
            Error::InvalidToken(token) => write!(f, "invalid byte token ({token:#x})"),
            Error::InvalidUtf8 => write!(f, "invalid UTF-8"),
            Error::InvalidValType(valtype) => write!(f, "invalid valtype ({valtype:#x})"),
            Error::OutOfOrderSection { before, after } => {
                write!(f, "out-of-order sections: {before:?} before {after:?}")
            }
            Error::Storage(err) => write!(f, "{err:?}"),
            Error::TooManyLocals(count) => {
                write!(f, "too many locals: at least {count} were specified")
            }
            Error::UnknownVersion(version) => write!(f, "unknown version ({version:#x})"),
        }
    }
}

impl<Storage: Stream> leb128::Error for Error<Storage> {
    fn invalid_leb128() -> Self {
        Error::InvalidLeb128
    }
}

impl<Storage: Stream> From<TryReserveError> for Error<Storage> {
    fn from(_: TryReserveError) -> Self {
        Error::AllocError
    }
}

pub(crate) struct Decoder<Storage: Stream> {
    stream: Storage,
}

impl<Storage: Stream> Decoder<Storage> {
    fn new(stream: Storage) -> Self {
        Self { stream }
    }

    // Pushes a context frame before a call, popping it if successful.
    fn with_context<F, R>(
        &mut self,
        context: &mut ContextStack,
        id: ContextId,
        f: F,
    ) -> Result<R, Error<Storage>>
    where
        F: FnOnce(&mut Self, &mut ContextStack) -> Result<R, Error<Storage>>,
    {
        let offset = self.stream.offset();
        if !context.push(id, offset) {
            return Err(Error::ExcessiveParsingDepth {
                context: id.into(),
                offset,
            });
        }
        let val = f(self, context)?;
        context.pop();
        Ok(val)
    }

    fn offset(&mut self) -> usize {
        self.stream.offset()
    }

    fn read_byte_raw(&mut self) -> Result<u8, Error<Storage>> {
        self.stream.read_byte().map_err(Error::Storage)
    }

    fn read_leb128_raw<T: Leb128>(&mut self) -> Result<T, Error<Storage>> {
        leb128::read(|| self.read_byte_raw())
    }

    fn read_zero_byte(&mut self, context: &mut ContextStack) -> Result<(), Error<Storage>> {
        self.with_context(context, ContextId::Byte, |decoder, _| {
            let byte = decoder.read_byte_raw()?;
            if byte == 0 {
                Ok(())
            } else {
                Err(Error::InvalidToken(byte))
            }
        })
    }

    fn read_exact_raw(&mut self, buf: &mut [u8]) -> Result<(), Error<Storage>> {
        self.stream.read_exact(buf).map_err(Error::Storage)
    }

    fn read_exact(
        &mut self,
        context: &mut ContextStack,
        buf: &mut [u8],
    ) -> Result<(), Error<Storage>> {
        self.with_context(context, ContextId::ReadingBytes, |decoder, _| {
            decoder.read_exact_raw(buf)
        })
    }

    fn skip_bytes(
        &mut self,
        context: &mut ContextStack,
        count: usize,
    ) -> Result<(), Error<Storage>> {
        self.with_context(context, ContextId::SkippingBytes, |decoder, _| {
            decoder.stream.skip_bytes(count).map_err(Error::Storage)
        })
    }

    fn read_bytes<A: Allocator>(
        &mut self,
        context: &mut ContextStack,
        count: usize,
        alloc: &A,
    ) -> Result<Box<[u8], A>, Error<Storage>> {
        let mut buf = Vec::new_in(alloc.clone());
        buf.try_reserve_exact(count)?;

        // Safety: With the previous call to try_reserve_exact(), there is
        // sufficient capacity and any uninitialized bytes will be overwritten
        // in the call to read_exact() below.
        unsafe { buf.set_len(count) };
        self.read_exact(context, &mut buf)?;
        Ok(buf.into_boxed_slice())
    }

    fn read<A: Allocator, T: Decodable<A> + Contextual>(
        &mut self,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<T, Error<Storage>> {
        self.with_context(context, T::ID, |decoder, context| {
            T::decode(decoder, context, alloc)
        })
    }

    fn read_bounded<T: BoundedDecodable + Contextual>(
        &mut self,
        context: &mut ContextStack,
    ) -> Result<T, Error<Storage>> {
        self.with_context(context, T::ID, |decoder, context| {
            T::decode(decoder, context)
        })
    }
}

// Types that can be decoded from a storage stream, possibly with allocation.
trait Decodable<A>: Sized
where
    A: Allocator,
{
    /// Parse this type from the binary stream.
    fn decode<Storage: Stream>(
        decoder: &mut Decoder<Storage>,
        context: &mut ContextStack,
        alloc: &A,
    ) -> Result<Self, Error<Storage>>;
}

// Types that can be decoded from a storage stream without allocation.
trait BoundedDecodable: Sized + Copy {
    fn decode<Storage: Stream>(
        decoder: &mut Decoder<Storage>,
        context: &mut ContextStack,
    ) -> Result<Self, Error<Storage>>;
}

impl<Bounded: BoundedDecodable, A: Allocator> Decodable<A> for Bounded {
    fn decode<Storage: Stream>(
        decoder: &mut Decoder<Storage>,
        context: &mut ContextStack,
        _: &A,
    ) -> Result<Self, Error<Storage>> {
        <Self as BoundedDecodable>::decode(decoder, context)
    }
}

/// Visitor pattern for processing custom sections during module parsing.
pub trait CustomSectionVisitor<A: Allocator> {
    /// Returns whether this visitor wants to process the custom section with the given name.
    fn should_visit(&self, name: &str) -> bool;
    /// Process a custom section. Only called if `should_visit` returned true.
    fn visit(&mut self, custom: CustomSection<A>);
}

/// No-op implementation of `CustomSectionVisitor` that skips all custom sections.
pub struct NoCustomSectionVisitor {}

impl<A: Allocator> CustomSectionVisitor<A> for NoCustomSectionVisitor {
    fn should_visit(&self, _: &str) -> bool {
        false
    }
    fn visit(&mut self, _: CustomSection<A>) {
        unreachable!()
    }
}

// Parse a WebAssembly module from a storage stream.
//
// # Arguments
// * `context` - Context stack for error reporting
// * `storage` - Data stream containing WASM binary
// * `customsec_visitor` - Handler for custom sections
// * `alloc` - Allocator for decoded data
pub(crate) fn decode_module<Storage, CustomSecVisitor, A>(
    storage: Storage,
    context: &mut ContextStack,
    customsec_visitor: &mut CustomSecVisitor,
    alloc: A,
) -> Result<Module<A>, Error<Storage>>
where
    Storage: Stream,
    CustomSecVisitor: CustomSectionVisitor<A>,
    A: Allocator,
{
    let mut decoder = Decoder::new(storage);
    decoder.read_bounded::<Magic>(context)?;
    let version: Version = decoder.read_bounded(context)?;

    let mut typesec = TypeSection::new(Vec::new_in(alloc.clone()));
    let mut importsec = ImportSection::new(Vec::new_in(alloc.clone()));
    let mut funcsec = FunctionSection::new(Vec::new_in(alloc.clone()));
    let mut tablesec = TableSection::new(Vec::new_in(alloc.clone()));
    let mut memsec = MemorySection::new(Vec::new_in(alloc.clone()));
    let mut globalsec = GlobalSection::new(Vec::new_in(alloc.clone()));
    let mut exportsec = ExportSection::new(Vec::new_in(alloc.clone()));
    let mut startsec = None;
    let mut elemsec = ElementSection::new(Vec::new_in(alloc.clone()));
    let mut datacountsec = None;
    let mut codesec = CodeSection::new(Vec::new_in(alloc.clone()));
    let mut datasec = DataSection::new(Vec::new_in(alloc.clone()));

    // The last section ID seen.
    let mut last_id = None;
    loop {
        // There is no in-band signal in the WASM format for the end of a
        // module. The best we can generically do is expect an EOF at a section
        // boundary.
        let id = decoder.read_bounded(context);
        if let Err(Error::Storage(ref err)) = id
            && Storage::is_eof(err)
        {
            break;
        }
        let id = id?;

        // Apart from custom sections, which can appear anywhere in the format,
        // sections must appear at most once and in order.
        if id != SectionId::Custom {
            if let Some(last_id) = last_id {
                if id <= last_id {
                    return Err(Error::OutOfOrderSection {
                        before: last_id,
                        after: id,
                    });
                }
                if id == last_id {
                    return Err(Error::DuplicateSection(id));
                }
            }
            last_id = Some(id);
        }

        let len: u32 = decoder.read_bounded(context)?;
        let offset_start = decoder.offset();
        match id {
            SectionId::Custom => {
                let (name, len) = {
                    let name_start = decoder.offset();
                    let name: Name<A> = decoder.read(context, &alloc)?;
                    let name_end = decoder.offset();

                    // If the name already exceeds the purported section length,
                    // we can break now and have the invalid length error
                    // reported below.
                    let len = len as usize;
                    if name_end - name_start > len {
                        break;
                    }
                    (name, len - (name_end - name_start))
                };
                if customsec_visitor.should_visit(name.as_ref()) {
                    let bytes = decoder.read_bytes(context, len, &alloc)?;
                    customsec_visitor.visit(CustomSection { name, bytes });
                } else {
                    decoder.skip_bytes(context, len)?;
                }
            }
            SectionId::Type => typesec = decoder.read(context, &alloc)?,
            SectionId::Import => importsec = decoder.read(context, &alloc)?,
            SectionId::Function => funcsec = decoder.read(context, &alloc)?,
            SectionId::Table => tablesec = decoder.read(context, &alloc)?,
            SectionId::Memory => memsec = decoder.read(context, &alloc)?,
            SectionId::Global => globalsec = decoder.read(context, &alloc)?,
            SectionId::Export => exportsec = decoder.read(context, &alloc)?,
            SectionId::Start => startsec = Some(decoder.read(context, &alloc)?),
            SectionId::Element => elemsec = decoder.read(context, &alloc)?,
            SectionId::Code => codesec = decoder.read(context, &alloc)?,
            SectionId::Data => datasec = decoder.read(context, &alloc)?,
            SectionId::DataCount => datacountsec = Some(decoder.read(context, &alloc)?),
        }
        let actual_section_len = decoder.offset() - offset_start;
        if actual_section_len != (len as usize) {
            return Err(Error::InvalidSectionLength {
                id,
                expected: len,
                actual: actual_section_len as u32,
            });
        }
    }

    Ok(Module {
        version,
        typesec,
        importsec,
        funcsec,
        tablesec,
        memsec,
        globalsec,
        exportsec,
        startsec,
        elemsec,
        datacountsec,
        codesec,
        datasec,
    })
}
