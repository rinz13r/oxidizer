using System;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using Native.Interop;
using Xunit;

namespace Oxidizer.Tests.E2E;

public class BindingsTests
{
    // --- Add (value type) ---

    [Fact]
    public void Add_ReturnsStructWithBothValues()
    {
        var result = MyBindings.Add(10, 20);
        Assert.Equal(10UL, result.X);
        Assert.Equal(20UL, result.Y);
    }

    [Fact]
    public void Add_ZeroValues()
    {
        var result = MyBindings.Add(0, 0);
        Assert.Equal(0UL, result.X);
        Assert.Equal(0UL, result.Y);
    }

    // --- GetNumbers (owned slice) ---

    [Fact]
    public void GetNumbers_ReturnsFiveElements()
    {
        using var numbers = MyBindings.GetNumbers();
        Assert.Equal(5, numbers.Length);
        Assert.Equal(new ulong[] { 1, 2, 3, 4, 5 }, numbers.AsSpan().ToArray());
    }

    [Fact]
    public void GetNumbers_Indexer()
    {
        using var numbers = MyBindings.GetNumbers();
        Assert.Equal(1UL, numbers[0]);
        Assert.Equal(5UL, numbers[4]);
    }

    // --- GetLargeArray (owned slice) ---

    [Fact]
    public void GetLargeArray_ReturnsSequence()
    {
        using var array = MyBindings.GetLargeArray(100);
        Assert.Equal(100, array.Length);
        var expected = new ulong[100];
        for (ulong i = 0; i < 100; i++) expected[i] = i;
        Assert.Equal(expected, array.AsSpan().ToArray());
    }

    [Fact]
    public void GetLargeArray_ZeroCount()
    {
        using var array = MyBindings.GetLargeArray(0);
        Assert.Equal(0, array.Length);
    }

    // --- SumNumbers (borrowed slice) ---

    [Fact]
    public unsafe void SumNumbers_SumsCorrectly()
    {
        var data = new ulong[] { 10, 20, 30 };
        fixed (ulong* ptr = data)
        {
            var slice = new FFISliceRaw { Ptr = (IntPtr)ptr, Len = (ulong)data.Length };
            var sum = MyBindings.SumNumbers(slice);
            Assert.Equal(60UL, sum);
        }
    }

    [Fact]
    public void SumNumbers_EmptySlice()
    {
        var slice = new FFISliceRaw { Ptr = IntPtr.Zero, Len = 0 };
        var sum = MyBindings.SumNumbers(slice);
        Assert.Equal(0UL, sum);
    }

    // --- WithData (slice callback) ---

    [Fact]
    public void WithData_CallbackReceivesCorrectData()
    {
        ulong[]? received = null;
        MyBindings.WithData(span =>
        {
            received = span.ToArray();
        });
        Assert.NotNull(received);
        Assert.Equal(new ulong[] { 1, 2, 3, 4, 5 }, received);
    }

    [Fact]
    public void WithData_CallbackSumIsCorrect()
    {
        ulong sum = 0;
        MyBindings.WithData(span =>
        {
            foreach (var v in span) sum += v;
        });
        Assert.Equal(15UL, sum);
    }

    // --- Heap-allocated types ---

    [Fact]
    public void HeapAllocCheck1_ReturnsHandle()
    {
        using var handle = MyBindings.HeapAllocCheck1();
        Assert.NotNull(handle);
    }

    [Fact]
    public void HeapSum_Returns30()
    {
        using var handle = MyBindings.HeapAllocCheck1();
        var sum = MyBindings.HeapSum(handle);
        Assert.Equal(30UL, sum);
    }

    [Fact]
    public void HeapAllocCheck2_ConsumesOwnership()
    {
        var handle = MyBindings.HeapAllocCheck1();
        MyBindings.HeapAllocCheck2(handle);
    }

    // --- Async functions ---

    [Fact]
    public async Task CheckAsync1_Returns42()
    {
        var result = await MyBindings.CheckAsync1(0).WaitAsync(TimeSpan.FromSeconds(10));
        Assert.Equal(42.0, result);
    }

    [Fact]
    public async Task CheckAsync2_Returns42()
    {
        var result = await MyBindings.CheckAsync2(0).WaitAsync(TimeSpan.FromSeconds(10));
        Assert.Equal(42UL, result);
    }

    [Fact]
    public async Task HeapAllocCheckAsync_ReturnsValidHandle()
    {
        using var handle = await MyBindings.HeapAllocCheckAsync().WaitAsync(TimeSpan.FromSeconds(10));
        var sum = MyBindings.HeapSum(handle);
        Assert.Equal(30UL, sum);
    }

    [Fact]
    public async Task MultipleAsyncCalls_RunConcurrently()
    {
        var sw = Stopwatch.StartNew();
        var tasks = new[]
        {
            MyBindings.CheckAsync1(0),
            MyBindings.CheckAsync1(0),
            MyBindings.CheckAsync1(0),
        };
        var results = await Task.WhenAll(tasks).WaitAsync(TimeSpan.FromSeconds(10));
        sw.Stop();

        foreach (var r in results)
            Assert.Equal(42.0, r);

        Assert.True(sw.Elapsed.TotalSeconds < 3.0, $"Expected < 3s but took {sw.Elapsed.TotalSeconds:F2}s");
    }
}
