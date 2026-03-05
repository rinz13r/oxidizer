import asyncio
import ctypes
import time

import pytest

from Generated import (
    FFISliceRaw,
    OwnedSliceHandle,
    add,
    check_async_1,
    check_async_2,
    get_large_array,
    get_numbers,
    heap_alloc_check_1,
    heap_alloc_check_2,
    heap_alloc_check_async,
    heap_sum,
    sum_numbers,
    with_data,
)


# --- add (value type) ---


def test_add_returns_struct():
    result = add(10, 20)
    assert result.x == 10
    assert result.y == 20


def test_add_zero_values():
    result = add(0, 0)
    assert result.x == 0
    assert result.y == 0


# --- get_numbers (owned slice) ---


def test_get_numbers_returns_five():
    with get_numbers() as nums:
        assert len(nums) == 5
        assert nums.to_list() == [1, 2, 3, 4, 5]


def test_get_numbers_indexer():
    with get_numbers() as nums:
        assert nums[0] == 1
        assert nums[4] == 5


def test_get_numbers_index_out_of_range():
    with get_numbers() as nums:
        with pytest.raises(IndexError):
            _ = nums[5]


# --- get_large_array (owned slice) ---


def test_get_large_array_returns_sequence():
    with get_large_array(100) as arr:
        assert len(arr) == 100
        assert arr.to_list() == list(range(100))


def test_get_large_array_zero_count():
    with get_large_array(0) as arr:
        assert len(arr) == 0


# --- OwnedSliceHandle dispose behaviour ---


def test_owned_slice_disposed_raises():
    nums = get_numbers()
    nums.dispose()
    with pytest.raises(RuntimeError):
        _ = nums[0]


# --- sum_numbers (borrowed slice) ---


def test_sum_numbers_sums_correctly():
    data = (ctypes.c_uint64 * 3)(10, 20, 30)
    raw = FFISliceRaw(ptr=ctypes.cast(data, ctypes.c_void_p), len=3)
    result = sum_numbers(raw)
    assert result == 60


def test_sum_numbers_single_element():
    data = (ctypes.c_uint64 * 1)(42)
    raw = FFISliceRaw(ptr=ctypes.cast(data, ctypes.c_void_p), len=1)
    result = sum_numbers(raw)
    assert result == 42


# --- with_data (slice callback) ---


def test_with_data_callback_data():
    received = []

    def on_data(data):
        received.extend(data)

    with_data(on_data)
    assert received == [1, 2, 3, 4, 5]


def test_with_data_callback_sum():
    total = 0

    def on_data(data):
        nonlocal total
        total = sum(data)

    with_data(on_data)
    assert total == 15


# --- Heap-allocated types ---


def test_heap_alloc_returns_handle():
    with heap_alloc_check_1() as handle:
        assert handle._raw.ptr is not None


def test_heap_sum_returns_30():
    with heap_alloc_check_1() as handle:
        result = heap_sum(handle)
        assert result == 30


def test_heap_alloc_check_2_consumes():
    handle = heap_alloc_check_1()
    heap_alloc_check_2(handle)


# --- Async functions ---


@pytest.mark.asyncio
async def test_check_async_1_returns_42():
    result = await asyncio.wait_for(check_async_1(0), timeout=10)
    assert result == pytest.approx(42.0)


@pytest.mark.asyncio
async def test_check_async_2_returns_42():
    result = await asyncio.wait_for(check_async_2(0), timeout=10)
    assert result == 42


@pytest.mark.asyncio
async def test_heap_alloc_check_async():
    handle = await asyncio.wait_for(heap_alloc_check_async(), timeout=10)
    with handle:
        result = heap_sum(handle)
        assert result == 30


@pytest.mark.asyncio
async def test_check_async_1_ignores_param():
    result = await asyncio.wait_for(check_async_1(999), timeout=10)
    assert result == pytest.approx(42.0)


@pytest.mark.asyncio
async def test_multiple_async_concurrent():
    start = time.monotonic()
    results = await asyncio.wait_for(
        asyncio.gather(check_async_1(0), check_async_1(0), check_async_1(0)),
        timeout=10,
    )
    elapsed = time.monotonic() - start

    for r in results:
        assert r == pytest.approx(42.0)

    assert elapsed < 3.0, f"Expected < 3s but took {elapsed:.2f}s"
