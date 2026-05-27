# Oxidizer

Language-agnostic Rust FFI binding generator.
Annotate Rust types/functions once, collect metadata in a `Registry`, then generate bindings for one or more target languages.

First-party generators in this repository:

- **C#** (`oxidizer_csgen`)
- **Python** (`oxidizer_pygen`)

## Features

- **Language-agnostic core** - reflection metadata and FFI primitives are shared across generators
- **Multi-language codegen** - generate C# and/or Python from the same Rust API surface
- **Async support** - Rust async functions map to C# `Task<T>` and Python `async def`
- **Ownership-aware FFI types**:
  - `Owned<T>` - opaque owned handle
  - `OwnedSlice<T>` - transfer `Vec<T>` ownership across FFI
  - `FFISlice<T>` / `FFISliceMut<T>` - borrowed slice input from caller
  - `SliceCallback<T>` - scoped callback access to Rust-owned slice data

## Installation

Add Oxidizer from GitHub plus whichever generators you need:

```toml
[dependencies]
oxidizer = { git = "https://github.com/rinz13r/oxidizer", package = "oxidizer" }

[build-dependencies]
oxidizer_csgen = { git = "https://github.com/rinz13r/oxidizer", package = "oxidizer_csgen" } # C# generator (optional)
oxidizer_pygen = { git = "https://github.com/rinz13r/oxidizer", package = "oxidizer_pygen" } # Python generator (optional)
```

## Quick Start

### 1. Annotate Rust types and functions

```rust
use oxidizer::prelude::*;

#[ffi_type]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[ffi_function]
fn add_points(a: Point, b: Point) -> Point {
    Point {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}
```

### 2. Register API and generate bindings

Typically done in `build.rs` (or a dedicated bindings crate):

```rust
use oxidizer::Registry;
use oxidizer_csgen::CSharpGenerator;
use oxidizer_pygen::PythonGenerator;

fn native_library_filename(base_name: &str) -> String {
    match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("windows") => format!("{base_name}.dll"),
        Ok("macos") => format!("lib{base_name}.dylib"),
        _ => format!("lib{base_name}.so"),
    }
}

fn main() {
    let mut registry = Registry::new();
    registry
        .register_type::<Point>()
        .register_function::<add_points>();

    // C#
    let csharp_code = CSharpGenerator::builder()
        .library_name("my_lib")
        .namespace("MyCompany.Interop")
        .build()
        .generate_csharp(&registry);
    std::fs::write("Generated.cs", csharp_code).unwrap();

    // Python
    let python_code = PythonGenerator::builder()
        .library_name(native_library_filename("my_lib"))
        .build()
        .generate_python(&registry);
    std::fs::write("Generated.py", python_code).unwrap();
}
```

### 3. Consume generated bindings

**C#**

```csharp
var result = Bindings.AddPoints(
    new Point { X = 1.0, Y = 2.0 },
    new Point { X = 3.0, Y = 4.0 }
);
Console.WriteLine($"Result: ({result.X}, {result.Y})");
```

**Python**

```python
from Generated import Point, add_points

result = add_points(Point(x=1.0, y=2.0), Point(x=3.0, y=4.0))
print(f"Result: ({result.x}, {result.y})")
```

## Async Functions

Rust async functions are exported as callback-based FFI and wrapped into idiomatic target APIs:

```rust
use oxidizer::prelude::*;
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

static RT: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());

#[ffi_function(RT)]
async fn fetch_data(id: u64) -> u64 {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    id * 2
}
```

- C#: `Task<ulong>` (e.g. `await Bindings.FetchData(42)`)
- Python: coroutine (e.g. `await fetch_data(42)`)

## Marker Types (Opaque Handles)

Use `#[ffi_type(marker)]` for opaque Rust types that should not expose fields in generated bindings.
Values cross the boundary as `Owned<T>`.

Generated API shape:

- C#: `OwnedHandle<T>` with `IDisposable`
- Python: `OwnedHandle` with context-manager semantics (`with ...:`)

## Slice Types

- `OwnedSlice<T>`: transfer owned arrays from Rust to caller
- `FFISlice<T>` / `FFISliceMut<T>`: pass borrowed caller memory into Rust
- `SliceCallback<T>`: invoke caller callback with scoped slice access

Generated wrappers expose ergonomic accessors such as spans/indexers in C# and helper handle classes in Python.

## Crate Structure

| Crate | Purpose |
|-------|---------|
| `oxidizer` | Facade crate re-exporting core/macro/utils APIs |
| `oxidizer_core` | Core traits (`ReflectType`, `ReflectFunction`) and metadata (`TypeInfo`, `FunctionInfo`, `TypeKind`) |
| `oxidizer_macro` | Proc macros: `#[ffi_type]`, `#[ffi_function]`, `#[ffi_type(marker)]` |
| `oxidizer_utils` | FFI primitives: `Owned`, `OwnedSlice`, `FFISlice`, `FFISliceMut`, `SliceCallback` |
| `oxidizer_csgen` | C# generator (`CSharpGenerator`) |
| `oxidizer_pygen` | Python generator (`PythonGenerator`) |

## Building and Testing

```bash
cargo build                  # Build default workspace members
cargo build --workspace      # Build all crates including E2E crates
cargo test --workspace       # Run all Rust tests
cargo xtask test             # Run unit + E2E tests through the cross-platform runner
cargo xtask test unit        # Run Rust tests only
cargo xtask generate-bindings # Generate E2E C# and Python bindings
```

For full E2E (Rust + generated C# + generated Python):

```bash
cargo xtask test e2e
```

## E2E Layout

```text
tests/e2e/
  rust_lib/            # Rust cdylib with exported FFI API
  bindings-generator/  # Generates target/generated/e2e/Generated.cs + Generated.py
  dotnet/              # C# integration tests
  python/              # Python integration tests
```

## C# Type Mapping (selected)

| Rust Type | C# Type |
|-----------|---------|
| `u8`, `u16`, `u32`, `u64` | `byte`, `ushort`, `uint`, `ulong` |
| `i8`, `i16`, `i32`, `i64` | `sbyte`, `short`, `int`, `long` |
| `f32`, `f64` | `float`, `double` |
| `bool` | `byte` |
| `*const T`, `*mut T` | `IntPtr` |
| `#[ffi_type] struct` | `struct` (`LayoutKind.Sequential`) |
| `Owned<T>` | `OwnedHandle<T>` |
| `OwnedSlice<T>` | `OwnedSliceHandle<T>` |
| `SliceCallback<T>` | `Action<ReadOnlySpan<T>>` |

Python bindings use `ctypes` FFI representations plus ergonomic wrapper classes for owned handles/slices.

## License

MIT
