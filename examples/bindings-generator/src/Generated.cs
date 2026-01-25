using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Threading.Tasks;

class Registrar_double
{
    public static readonly Registrar_double Instance = new();

    public delegate void CallbackDelegate(ulong id, double result);

    private readonly Dictionary<ulong, Action<double>> registrations = new();
    private ulong id = 0;
    private readonly object lockObj = new();

    private Registrar_double()
    {
    }

    public ulong Register(Action<double> callback)
    {
        ulong currentId;

        lock (lockObj)
        {
            currentId = id;
            registrations[currentId] = callback;
            id++;
        }

        return currentId;
    }

    public static void Callback(ulong id, double result)
    {
        if (Instance.registrations.TryGetValue(id, out var callback))
        {
            lock (Instance.lockObj)
            {
                Instance.registrations.Remove(id);
            }
            callback(result);
        }
    }
}

class Registrar_HeapAllocatedRaw
{
    public static readonly Registrar_HeapAllocatedRaw Instance = new();

    public delegate void CallbackDelegate(ulong id, HeapAllocatedRaw result);

    private readonly Dictionary<ulong, Action<HeapAllocatedRaw>> registrations = new();
    private ulong id = 0;
    private readonly object lockObj = new();

    private Registrar_HeapAllocatedRaw()
    {
    }

    public ulong Register(Action<HeapAllocatedRaw> callback)
    {
        ulong currentId;

        lock (lockObj)
        {
            currentId = id;
            registrations[currentId] = callback;
            id++;
        }

        return currentId;
    }

    public static void Callback(ulong id, HeapAllocatedRaw result)
    {
        if (Instance.registrations.TryGetValue(id, out var callback))
        {
            lock (Instance.lockObj)
            {
                Instance.registrations.Remove(id);
            }
            callback(result);
        }
    }
}

class Registrar_ulong
{
    public static readonly Registrar_ulong Instance = new();

    public delegate void CallbackDelegate(ulong id, ulong result);

    private readonly Dictionary<ulong, Action<ulong>> registrations = new();
    private ulong id = 0;
    private readonly object lockObj = new();

    private Registrar_ulong()
    {
    }

    public ulong Register(Action<ulong> callback)
    {
        ulong currentId;

        lock (lockObj)
        {
            currentId = id;
            registrations[currentId] = callback;
            id++;
        }

        return currentId;
    }

    public static void Callback(ulong id, ulong result)
    {
        if (Instance.registrations.TryGetValue(id, out var callback))
        {
            lock (Instance.lockObj)
            {
                Instance.registrations.Remove(id);
            }
            callback(result);
        }
    }
}

[StructLayout(LayoutKind.Sequential)]
public struct HeapAllocatedRaw
{
    public IntPtr Ptr;
    public IntPtr DropFn;
}

/// <summary>
/// Type-safe wrapper for heap-allocated Rust objects.
/// Implements IDisposable to ensure proper cleanup of native resources.
/// </summary>
public sealed class HeapHandle<T> : IDisposable
{
    private HeapAllocatedRaw _raw;
    private bool _disposed;

    internal HeapHandle(HeapAllocatedRaw raw) => _raw = raw;
    internal HeapAllocatedRaw Raw => _raw;

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        if (_raw.Ptr != IntPtr.Zero)
        {
            Bindings.DropHeapAllocated(_raw);
            _raw.Ptr = IntPtr.Zero;
        }
    }
}

/// <summary>Type-erased borrowed slice for FFI boundary.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct FFISliceRaw
{
    public IntPtr Ptr;
    public nuint Len;
}

/// <summary>Type-erased owned slice for FFI boundary.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct OwnedSliceRaw
{
    public IntPtr Ptr;
    public nuint Len;
    public nuint Capacity;
    public nuint ElementSize;
    public IntPtr DropFn;
}

/// <summary>
/// Read-only view into a borrowed Rust slice.
/// The underlying data is owned by Rust and must not be modified.
/// </summary>
public readonly ref struct ReadOnlySliceHandle<T> where T : unmanaged
{
    private readonly FFISliceRaw _raw;

    internal ReadOnlySliceHandle(FFISliceRaw raw) => _raw = raw;

    public int Length => (int)_raw.Len;

    public unsafe ReadOnlySpan<T> AsSpan()
    {
        if (_raw.Ptr == IntPtr.Zero || _raw.Len == 0)
            return ReadOnlySpan<T>.Empty;
        return new ReadOnlySpan<T>((void*)_raw.Ptr, (int)_raw.Len);
    }
}

/// <summary>
/// Mutable view into a borrowed Rust slice.
/// </summary>
public readonly ref struct SliceHandle<T> where T : unmanaged
{
    private readonly FFISliceRaw _raw;

    internal SliceHandle(FFISliceRaw raw) => _raw = raw;

    public int Length => (int)_raw.Len;

    public unsafe Span<T> AsSpan()
    {
        if (_raw.Ptr == IntPtr.Zero || _raw.Len == 0)
            return Span<T>.Empty;
        return new Span<T>((void*)_raw.Ptr, (int)_raw.Len);
    }
}

/// <summary>
/// Owned array transferred from Rust.
/// Implements IDisposable to ensure proper cleanup of native resources.
/// </summary>
public sealed class OwnedArray<T> : IDisposable where T : unmanaged
{
    private OwnedSliceRaw _raw;
    private bool _disposed;

    internal OwnedArray(OwnedSliceRaw raw) => _raw = raw;
    internal OwnedSliceRaw Raw => _raw;

    public int Length => (int)_raw.Len;

    public unsafe ReadOnlySpan<T> AsSpan()
    {
        if (_disposed) throw new ObjectDisposedException(nameof(OwnedArray<T>));
        if (_raw.Ptr == IntPtr.Zero || _raw.Len == 0)
            return ReadOnlySpan<T>.Empty;
        return new ReadOnlySpan<T>((void*)_raw.Ptr, (int)_raw.Len);
    }

    public T this[int index]
    {
        get
        {
            if (_disposed) throw new ObjectDisposedException(nameof(OwnedArray<T>));
            if (index < 0 || index >= (int)_raw.Len)
                throw new IndexOutOfRangeException();
            unsafe { return ((T*)_raw.Ptr)[index]; }
        }
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        if (_raw.Ptr != IntPtr.Zero)
        {
            Bindings.DropOwnedSlice(_raw);
            _raw.Ptr = IntPtr.Zero;
        }
    }
}

[StructLayout(LayoutKind.Sequential)]
public struct FFITy
{
    public ulong X;
    public ulong Y;
}

/// <summary>Marker struct for heap-allocated FFIHeapTy instances.</summary>
public struct FFIHeapTy { }

public static class Bindings
{
    [DllImport("rust_lib.dll", EntryPoint = "drop_heap_allocated", CallingConvention = CallingConvention.Cdecl)]
    public static extern void DropHeapAllocated(HeapAllocatedRaw ha);

    [DllImport("rust_lib.dll", EntryPoint = "drop_owned_slice", CallingConvention = CallingConvention.Cdecl)]
    public static extern void DropOwnedSlice(OwnedSliceRaw os);

    [DllImport("rust_lib.dll", EntryPoint = "add", CallingConvention = CallingConvention.Cdecl)]
    public static extern FFITy Add(ulong x, ulong y);

    public static async Task<double> CheckAsync1(int _param)
    {
        var tcs = new TaskCompletionSource<double>();

        var id = Registrar_double.Instance.Register(
            (double res) =>
            {
                tcs.SetResult(res);
            });

        CheckAsync1Internal(id, _param, Registrar_double.Callback);

        return await tcs.Task;
    }

    [DllImport("rust_lib.dll", EntryPoint = "check_async_1", CallingConvention = CallingConvention.Cdecl)]
    private static extern void CheckAsync1Internal(ulong id, int _param, Registrar_double.CallbackDelegate cb);

    public static async Task<ulong> CheckAsync2(int _param)
    {
        var tcs = new TaskCompletionSource<ulong>();

        var id = Registrar_ulong.Instance.Register(
            (ulong res) =>
            {
                tcs.SetResult(res);
            });

        CheckAsync2Internal(id, _param, Registrar_ulong.Callback);

        return await tcs.Task;
    }

    [DllImport("rust_lib.dll", EntryPoint = "check_async_2", CallingConvention = CallingConvention.Cdecl)]
    private static extern void CheckAsync2Internal(ulong id, int _param, Registrar_ulong.CallbackDelegate cb);

    [DllImport("rust_lib.dll", EntryPoint = "heap_alloc_check_1", CallingConvention = CallingConvention.Cdecl)]
    private static extern HeapAllocatedRaw HeapAllocCheck1Internal();

    public static HeapHandle<FFIHeapTy> HeapAllocCheck1()
    {
        return new HeapHandle<FFIHeapTy>(HeapAllocCheck1Internal());
    }

    [DllImport("rust_lib.dll", EntryPoint = "heap_alloc_check_2", CallingConvention = CallingConvention.Cdecl)]
    private static extern void HeapAllocCheck2Internal(HeapAllocatedRaw _param);

    public static void HeapAllocCheck2(HeapHandle<FFIHeapTy> _param)
    {
        HeapAllocCheck2Internal(_param.Raw);
    }

    public static async Task<HeapHandle<FFIHeapTy>> HeapAllocCheckAsync()
    {
        var tcs = new TaskCompletionSource<HeapAllocatedRaw>();

        var id = Registrar_HeapAllocatedRaw.Instance.Register(
            (HeapAllocatedRaw res) =>
            {
                tcs.SetResult(res);
            });

        HeapAllocCheckAsyncInternal(id, Registrar_HeapAllocatedRaw.Callback);

        return new HeapHandle<FFIHeapTy>(await tcs.Task);
    }

    [DllImport("rust_lib.dll", EntryPoint = "heap_alloc_check_async", CallingConvention = CallingConvention.Cdecl)]
    private static extern void HeapAllocCheckAsyncInternal(ulong id, Registrar_HeapAllocatedRaw.CallbackDelegate cb);

    [DllImport("rust_lib.dll", EntryPoint = "get_numbers", CallingConvention = CallingConvention.Cdecl)]
    private static extern OwnedSliceRaw GetNumbersInternal();

    public static OwnedArray<ulong> GetNumbers()
    {
        return new OwnedArray<ulong>(GetNumbersInternal());
    }

    [DllImport("rust_lib.dll", EntryPoint = "sum_numbers", CallingConvention = CallingConvention.Cdecl)]
    public static extern ulong SumNumbers(FFISliceRaw data);

    [DllImport("rust_lib.dll", EntryPoint = "get_large_array", CallingConvention = CallingConvention.Cdecl)]
    private static extern OwnedSliceRaw GetLargeArrayInternal(ulong count);

    public static OwnedArray<ulong> GetLargeArray(ulong count)
    {
        return new OwnedArray<ulong>(GetLargeArrayInternal(count));
    }

}
