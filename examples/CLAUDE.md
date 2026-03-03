# Examples

Demonstrates oxidizer's Rust-to-C# and Rust-to-Python FFI workflows.

## Components

### rust_lib/
Rust library with FFI exports. Builds as `cdylib` -> `target/debug/rust_lib.dll`

Key files:
- `src/lib.rs` - FFI types and functions
- `src/init.rs` - Tokio runtime setup (`RT`)

Key details:
- Uses `#[ffi_type]` for `FFITy`, `#[ffi_type(marker)]` for `FFIHeapTy`
- Async functions use `#[ffi_function(RT)]` with tokio runtime
- `get_ffi_types_registry()` returns `Registry` with all registered types/functions
- Dependencies: `tokio` (async), `ctor` (init), `lazy_static` (runtime storage)

### bindings-generator/
Generates C# and Python bindings via `build.rs` (runs during `cargo build`).

Key files:
- `build.rs:10` - Calls `rust_lib::get_ffi_types_registry()`
- `build.rs:12` - DLL name hardcoded as `"rust_lib.dll"`
- `build.rs` - Output paths: `src/Generated.cs` and `src/Generated.py`

### DotnetApp/
.NET 9 console app consuming the generated C# bindings.

Key files:
- `DotnetApp.csproj:25` - Includes `../bindings-generator/src/Generated.cs`
- `DotnetApp.csproj:12-20` - Copies `rust_lib.dll` from target/
- Requires `AllowUnsafeBlocks` for pointer operations

### PythonApp/
Python script consuming the generated ctypes bindings.

Key files:
- `main.py` - Imports from `Generated.py`, demonstrates all FFI patterns
- Copies `rust_lib.dll` next to `Generated.py` at startup
- Uses `asyncio.run()` for async function demos

## Build & Run

### C# (.NET)

```powershell
./run.ps1
```

Or manually:
```
cargo build                       # Core oxidizer crates
cargo build -p rust_lib           # Build Rust DLL
cargo build -p bindings-generator # Generate src/Generated.cs + src/Generated.py
dotnet run --project examples/DotnetApp/DotnetApp.csproj
```

### Python

```
cargo build -p rust_lib           # Build Rust DLL
cargo build -p bindings-generator # Generate src/Generated.py
python examples/PythonApp/main.py
```

## Flow

```
rust_lib                    bindings-generator              DotnetApp / PythonApp
   |                               |                            |
   +- #[ffi_type]                  |                            |
   +- #[ffi_function]              |                            |
   +- get_ffi_types_registry() --->|                            |
   |                               +- CSharpGenerator           |
   |                               +- Generated.cs ------------>| (DotnetApp: P/Invoke)
   |                               +- PythonGenerator           |
   |                               +- Generated.py ------------>| (PythonApp: ctypes)
   |<-----------------------------------------------------------+
   |                         rust_lib.dll                       |
```
