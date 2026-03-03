# Oxidizer FFI Bindings Demo - Python Edition
# Demonstrates Rust-to-Python interop via generated ctypes bindings

import sys
import os
import shutil
import asyncio

# Add bindings-generator output to path
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
BINDINGS_DIR = os.path.join(SCRIPT_DIR, "..", "bindings-generator", "src")
sys.path.insert(0, BINDINGS_DIR)

# Copy rust_lib.dll next to Generated.py so ctypes.CDLL can find it
DLL_CANDIDATES = [
    os.path.join(SCRIPT_DIR, "..", "..", "target", "debug", "rust_lib.dll"),
    os.path.join(SCRIPT_DIR, "..", "..", "target", "release", "rust_lib.dll"),
]
DLL_DEST = os.path.join(BINDINGS_DIR, "rust_lib.dll")

for candidate in DLL_CANDIDATES:
    if os.path.exists(candidate):
        shutil.copy2(candidate, DLL_DEST)
        break
else:
    print("Error: rust_lib.dll not found. Run 'cargo build -p rust_lib' first.")
    sys.exit(1)

from Generated import (
    add,
    check_async_1,
    heap_alloc_check_1,
    heap_alloc_check_2,
    heap_alloc_check_async,
    get_large_array,
    with_data,
)


async def main():
    # --- Heap-Allocated Types ---
    # OwnedHandle provides type-safe, disposable wrappers for Rust heap allocations.
    # The 'with' statement ensures the Rust drop function is called when scope exits.

    with await heap_alloc_check_async() as heap_obj:
        heap_alloc_check_2(heap_obj)
        print(f"Heap-allocated object: {type(heap_obj).__name__}")

    # --- Value Types ---
    # Structs marked with #[ffi_type] are marshalled by value.

    result = add(10, 20)
    print(f"Add(10, 20) => x={result.x}, y={result.y}")

    # --- Owned Slices ---
    # OwnedSliceHandle wraps Rust Vec<T> with automatic cleanup.

    with get_large_array(12) as array:
        print(f"Array contents: {array.to_list()}")

    # --- Slice Callbacks ---
    # Pass a callable to receive a read-only view of Rust data.

    total = 0

    def on_data(data):
        nonlocal total
        total = sum(data)

    with_data(on_data)
    print(f"Sum from callback: {total}")

    # --- Async Functions ---

    result = await check_async_1(42)
    print(f"Async result: {result}")

    print("Done!")


if __name__ == "__main__":
    asyncio.run(main())
