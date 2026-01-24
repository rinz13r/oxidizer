// Example usage of heap-allocated types with type-safe HeapHandle<T>

// Create a heap-allocated FFIHeapTy instance
using var heapObj = Bindings.HeapAllocCheck();

Console.WriteLine("Created heap-allocated FFIHeapTy instance");
Console.WriteLine($"HeapHandle type: {heapObj.GetType().Name}");

// The object will be automatically disposed when leaving the using scope,
// which calls the Rust drop function to free the memory.

// Example of value type usage
var valueTy = Bindings.Add(10, 20);
Console.WriteLine($"Add result: x={valueTy.X}, y={valueTy.Y}");

Console.WriteLine("Done!");
