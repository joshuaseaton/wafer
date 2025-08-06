# Wafer

**W**eb **A**ssembly... **F**ast? (/**F**lexible?)... **E**mbeddable
**R**untime.

A WebAssembly parsing, validation, and runtime library focused on correctness,
flexibility, and performance

## Features

- **No-std compatible flexibility** - Works in embedded environments with
  optional standard library features (gated on the `std` feature). Generally
  represents environment-specific _choices_ as generics in interfaces (e.g.,
  WASM storage types or allocators)
- **Minimal, explicit, fallible allocation** - Dynamic allocations are kept to a
  minimum (insofar as that's possible when dealing with a binary format with
  packed, variable-length encodings), and made explicit by providing an
  allocator generic to any routine that needs them. Moreover, all allocation is
  fallible.
- **Flexibility**: Generic over allocators and storage types, and avoids the use
  of the standard library.
- **Spec-compliant validation** - Full validation of the 1.0 WASM binary format,
  validated against the WebAssembly spec.git's own conformance test suite.
- **Rich decoding error context** - Detailed error during parsing detailing the
  context frames down to where the error occurred (accomplished with a
  relatively small stack allocated context stack structure).

## Architecture

### Storage layer (`storage`)
Abstraction over binary data sources supporting
both in-memory buffers and streaming I/O. The `Stream` trait provides sequential
reading with error handling and EOF detection.

### Decoding (`decoding`)
Streaming decoding/parsing with contextual error reporting. Maintains a
context stack during decoding to provide detailed error locations. Handles all
WASM object types with proper validation. Re-encodes WASM expressions from
LEB128-encoded format into a naturally-aligned and repr(C)-encoded bytecode for
later performant execution. Uses lookup tables for efficient instruction
dispatch around operand re-encoding.

```rust
use wafer::{decode_module, storage::Buffer, decode::ContextStack, NoCustomSectionVisitor};

let wasm_bytes = include_bytes!("module.wasm");
let storage = Buffer::new(wasm_bytes);
let mut context = ContextStack::default();
let mut visitor = NoCustomSectionVisitor;

let module = decode_module(&mut context, storage, &mut visitor, std::alloc::Global)
    .with_context(&context)?;

println!("Functions: {}", module.funcsec.as_ref().map_or(0, |s| s.len()));
```

## Validation (TODO)

## Execution (TODO)

## License

[MIT](LICENSE)
