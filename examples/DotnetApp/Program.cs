// Oxidizer FFI Bindings Demo
// Demonstrates Rust-to-C# interop via generated P/Invoke bindings

// --- Heap-Allocated Types ---
// HeapHandle<T> provides type-safe, disposable wrappers for Rust heap allocations.
// The 'using' statement ensures the Rust drop function is called when scope exits.

using var heapObj = await Bindings.HeapAllocCheckAsync();
Bindings.HeapAllocCheck2(heapObj);
Console.WriteLine($"Heap-allocated object: {heapObj.GetType().Name}");

// --- Value Types ---
// Structs marked with #[ffi_type] are marshalled by value.

var result = Bindings.Add(10, 20);
Console.WriteLine($"Add(10, 20) => x={result.X}, y={result.Y}");

// --- Owned Slices ---
// OwnedSlice<T> wraps Rust Vec<T> with automatic cleanup.

using var array = Bindings.GetLargeArray(12);
Console.WriteLine($"Array contents: {string.Join(", ", array.AsSpan().ToArray())}");

// --- Slice Callbacks ---
// Pass a delegate to receive a read-only view of Rust data without copying.

ulong sum = 0;
Bindings.WithData(slice =>
{
    foreach (var value in slice)
        sum += value;
});
Console.WriteLine($"Sum from callback: {sum}");

Console.WriteLine("Done!");
