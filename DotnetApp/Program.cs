using System.Runtime.InteropServices;

var x = await Bindings.CheckAsync1(10);

Console.WriteLine($"1 + 2 = {x}");
