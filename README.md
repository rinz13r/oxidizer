# Oxidizer

Rust-to-C# FFI binding generator. Annotate Rust types and functions with proc macros, then generate type-safe C# P/Invoke bindings automatically.

## Features

- **Zero boilerplate** - Annotate Rust code, generate C# bindings in one step
- **Async support** - Rust async functions become C# `Task<T>` methods
- **Owned types** - Safe transfer of heap-allocated Rust objects with automatic cleanup via `IDisposable`
- **Slice types** - Multiple patterns for different ownership scenarios:
  - `OwnedSlice<T>` - Transfer `Vec` ownership to C#
  - `FFISlice<T>` / `FFISliceMut<T>` - Borrow slices from C#
  - `SliceCallback<T>` - Safe scoped access to Rust data via callback
- **Type safety** - Generic wrappers preserve type information across the FFI boundary

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
oxidizer = "0.1"

[build-dependencies]
oxidizer_csgen = "0.1"
```

## Quick Start

### 1. Annotate Rust Types and Functions

```rust
use oxidizer::prelude::*;

#[ffi_type]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[ffi_function]
fn add_points(a: Point, b: Point) -> Point {
    Point { x: a.x + b.x, y: a.y + b.y }
}
```

### 2. Generate C# Bindings

Create a registry and generate bindings (typically in `build.rs` or a separate crate):

```rust
use oxidizer::Registry;
use oxidizer_csgen::CSharpGenerator;

fn main() {
    let mut registry = Registry::new();
    registry
        .register_type::<Point>()
        .register_function::<add_points>();

    let cs_code = CSharpGenerator::generate_csharp(&registry, "my_lib");
    std::fs::write("Generated.cs", cs_code).unwrap();
}
```

### 3. Use in C#

```csharp
var result = Bindings.AddPoints(
    new Point { X = 1.0, Y = 2.0 },
    new Point { X = 3.0, Y = 4.0 }
);
Console.WriteLine($"Result: ({result.X}, {result.Y})");
```

## Async Functions

Rust async functions are transformed into callback-based FFI and wrapped as C# `Task<T>`:

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

C# receives an idiomatic async API:

```csharp
ulong result = await Bindings.FetchData(42);
```

## Marker Types (Opaque Handles)

Use `#[ffi_type(marker)]` for types that should be opaque to C#. The generated C# code contains an empty marker struct, and values are wrapped with `Owned<T>` on the Rust side:

```rust
#[ffi_type(marker)]
pub struct DatabaseConnection {
    handle: *mut c_void,
    // ... internal state
}

#[ffi_function]
fn connect(url: &str) -> Owned<DatabaseConnection> {
    Owned::new(DatabaseConnection { /* ... */ })
}

#[ffi_function]
fn query(conn: Owned<DatabaseConnection>) {
    // Use connection...
}
```

C# receives a type-safe `OwnedHandle<T>` with automatic cleanup:

```csharp
using var conn = Bindings.Connect("localhost");
Bindings.Query(conn);
// Automatically calls Rust drop when disposed
```

## Slice Types

### OwnedSlice - Transfer Vec Ownership

```rust
#[ffi_function]
fn get_numbers() -> OwnedSlice<u64> {
    OwnedSlice::from_vec(vec![1, 2, 3, 4, 5])
}
```

```csharp
using var numbers = Bindings.GetNumbers();
foreach (var n in numbers.AsSpan()) {
    Console.WriteLine(n);
}
```

### FFISlice - Borrow from C#

```rust
#[ffi_function]
fn sum_numbers(data: FFISlice<u64>) -> u64 {
    unsafe { data.as_slice().iter().sum() }
}
```

```csharp
Span<ulong> data = stackalloc ulong[] { 1, 2, 3, 4, 5 };
ulong sum = Bindings.SumNumbers(data);
```

### SliceCallback - Safe Scoped Access

When Rust needs to provide slice data to C#, use `SliceCallback` for safe scoped access:

```rust
#[ffi_function]
fn with_data(callback: SliceCallback<u64>) {
    let data = vec![1, 2, 3, 4, 5];
    callback.call(&data);
}
```

```csharp
ulong sum = 0;
Bindings.WithData((ReadOnlySpan<ulong> slice) => {
    foreach (var n in slice) sum += n;
});
```

## Crate Structure

| Crate | Purpose |
|-------|---------|
| `oxidizer` | Facade crate re-exporting core/macro/utils APIs only |
| `oxidizer_core` | Core traits (`ReflectType`, `ReflectFunction`) and metadata types (`TypeInfo`, `FunctionInfo`) |
| `oxidizer_macro` | Proc macros: `#[ffi_type]`, `#[ffi_function]`, `#[ffi_type(marker)]` |
| `oxidizer_utils` | FFI primitives: `Owned`, `OwnedSlice`, `FFISlice`, `FFISliceMut`, `SliceCallback` |
| `oxidizer_csgen` | C# code generator (`CSharpGenerator`) |

## Building

```bash
cargo build                  # Build core crates
cargo build --workspace      # Build everything including examples
cargo test --workspace       # Run all tests
```

## Example Project

The `examples/` directory contains a complete workflow:

```
examples/
  rust_lib/           # Rust cdylib with FFI exports
  bindings-generator/ # Generates C# bindings via build.rs
  DotnetApp/          # .NET console app consuming the bindings
```

Run the example:

```powershell
# Build Rust library
cargo build -p rust_lib

# Generate C# bindings
cargo build -p bindings-generator

# Run .NET app
dotnet run --project examples/DotnetApp/DotnetApp.csproj
```

Or use the provided script:

```powershell
./examples/run.ps1
```

## Type Mapping

| Rust Type | C# Type |
|-----------|---------|
| `u8`, `u16`, `u32`, `u64` | `byte`, `ushort`, `uint`, `ulong` |
| `i8`, `i16`, `i32`, `i64` | `sbyte`, `short`, `int`, `long` |
| `f32`, `f64` | `float`, `double` |
| `bool` | `bool` |
| `usize` | `nuint` |
| `*const T`, `*mut T` | `nint` / `IntPtr` |
| `#[ffi_type] struct` | `struct` (LayoutKind.Sequential) |
| `Owned<T>` | `OwnedHandle<T>` |
| `OwnedSlice<T>` | `OwnedArray<T>` |
| `FFISlice<T>` | `ReadOnlySpan<T>` |
| `FFISliceMut<T>` | `Span<T>` |
| `SliceCallback<T>` | `SliceCallbackHandler<T>` |

## License

MIT
