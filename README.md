## Problems

1. Automatic bindings generation (Functions, Structs) from Rust to C# (or even other languages)
2. Async Handling (Atleast in C# to begin with)

## Why
1. Convenience and minimizing human errors. No manual developer intervention required to generate and maintain the bindings in various languages


## Macros
1. `#[ffi_type]`
    - Generates the WireType impl for the corresponding type
2. `#[ffi_function]`
    - Instead of a function, it generates a struct with the same name, for ease of use
    - Generates the WireFunction impl for hte corresponding function


## Handling Async
- Assuming that there's a static RT created. Ideally, we should be RT agnostic (tokio, smol, etc.)
- Maybe the symbol can be passed as parameter to the ffi_function macro in the following manner

```rust
#[ffi_function(RT)]
async fn call(arg0: T0) -> TRes {
    ...
}
```

- Intentional choice: Forget about other RTs, support just tokio for now.

- We then transform this function to use the RT to spawn a task.

```rust
fn call(id: u64, arg0: T0, cb: extern "C" fn (u64, TRes)) {
    // Original method pasted with just different function name
    async fn call_internal(arg0: T0) -> TRes {
        ...
    }

    // Task enqueued on RT which invokes cb after the call.
    RT.spawn(async move || {
        cb(id, call_internal(arg0).await);
    });
}
```

- On the C# side, we'll have 2 methods (one public and the other private)
```CSharp
class Bindings
{
    public static async Task<TRes> Call(T0 arg0)
    {
        var tcs = new TaskCompletionSource<TRes>();

        var id = Registrar_TRes.Instance.Register(
            (TRes res) =>
            {
                tcs.SetResult(res);
            });

        CallInternal(id, arg0, Registrar_TRes.Callback);

        return await tcs.Task;
    }

    [DllImport("rust_lib.dll", EntryPoint = "call", CallingConvention = CallingConvention.Cdecl)]
    private static void CallInternal(long id, T0 arg0, Registrar_TRes.CallbackDelegate cb);
}
```

- The Registrar class will be generated corresponding to each return type on the Rust side, since we need a static method at compilation time, which can be passed to the Rust side.
Generic types are instantiated at runtime and location of static method will not be available.

- 
