# Oxidizer

Rust-to-C# FFI binding generator. Annotate Rust types and functions with macros, then generate C# P/Invoke bindings automatically.

## Crate Structure

| Crate | Purpose |
|-------|---------|
| `oxidizer_core` | Core traits (`WireType`, `WireFunction`) and metadata types (`TypeInfo`, `FunctionInfo`, `TypeKind`) |
| `oxidizer_macro` | Proc macros: `#[ffi_type]`, `#[ffi_function]`, `#[ffi_type(marker)]` |
| `oxidizer_utils` | FFI primitives: `Owned`, `OwnedSlice`, `FFISlice`, `FFISliceMut`, `SliceCallback` |
| `oxidizer_csgen` | C# code generator (`CSharpGenerator`) |
| `oxidizer` | Facade crate re-exporting everything; `csgen` feature enables code generation |

## Key Concepts

- `#[ffi_type]` generates `WireType` impl, exposing struct layout as `TypeInfo`
- `#[ffi_type(marker)]` marks types as markers (empty C# struct, wrapped with `Owned<T>` in Rust)
- `#[ffi_function]` generates `WireFunction` impl and `extern "C"` wrapper
- `#[ffi_function(RT)]` for async functions - takes a tokio runtime identifier
- `Registry` collects types/functions via `.register_type::<T>()` and `.register_function::<F>()`
- `CSharpGenerator::generate_csharp(&registry)` produces C# code string

## Build

```
cargo build                  # Builds default-members (core crates only)
cargo build --workspace      # Builds everything including examples
cargo test --workspace       # Run all tests
```

## Examples

See `examples/CLAUDE.md` for the rust_lib -> bindings-generator -> DotnetApp workflow.
