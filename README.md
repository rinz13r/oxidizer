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