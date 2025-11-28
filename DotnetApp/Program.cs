using System.Runtime.InteropServices;

// var x = await Bindings.CheckAsync1(10);
var ha = Bindings.HeapAllocCheck();

Bindings.DropHeapAllocated(ha);
