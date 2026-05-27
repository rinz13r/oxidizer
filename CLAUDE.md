# Oxidizer

Rust-to-C# FFI binding generator. Annotate Rust types and functions with macros, then generate C# P/Invoke bindings automatically.

## Crate Structure

| Crate | Purpose |
|-------|---------|
| `oxidizer_core` | Core traits (`ReflectType`, `ReflectFunction`) and metadata types (`TypeInfo`, `FunctionInfo`, `TypeKind`) |
| `oxidizer_macro` | Proc macros: `#[ffi_type]`, `#[ffi_function]`, `#[ffi_type(marker)]` |
| `oxidizer_utils` | FFI primitives: `Owned`, `OwnedSlice`, `FFISlice`, `FFISliceMut`, `SliceCallback` |
| `oxidizer_csgen` | C# code generator (`CSharpGenerator`) |
| `oxidizer` | Facade crate re-exporting core/macro/utils APIs only |

## Key Concepts

- `#[ffi_type]` generates `ReflectType` impl, exposing struct layout as `TypeInfo`
- `#[ffi_type(marker)]` marks types as markers (empty C# struct, wrapped with `Owned<T>` in Rust)
- `#[ffi_function]` generates `ReflectFunction` impl and `extern "C"` wrapper
- `#[ffi_function(RT)]` for async functions - takes a tokio runtime identifier
- `Registry` collects types/functions via `.register_type::<T>()` and `.register_function::<F>()`
- `CSharpGenerator::generate_csharp(&registry)` produces C# code string

## Build

```
cargo build                  # Builds default-members (core crates only)
cargo build --workspace      # Builds everything including examples
cargo test --workspace       # Run all tests
```

## E2E Tests

See `tests/e2e/` — run with `cargo xtask test e2e`. Contains `rust_lib` (Rust FFI library), `bindings-generator` (generates C#/Python bindings), and `dotnet`/`python` test projects.
