use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, ItemFn, parse_macro_input};

/// A procedural macro to generate FFI type information for structs
///
/// Usage:
/// ```rust
/// #[ffi_type]
/// struct MyStruct {
///     field1: u32,
///     field2: i64,
/// }
/// ```
///
/// For heap-allocated types (marker structs in C#):
/// ```rust
/// #[ffi_type(heap)]
/// struct MyHeapType {
///     field1: u32,
/// }
/// ```
#[proc_macro_attribute]
pub fn ffi_type(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    // Parse the attribute to check for "heap"
    let is_heap = !attr.is_empty() && attr.to_string().trim() == "heap";

    let struct_name = &input.ident;
    let vis = &input.vis;

    // Generate the original struct and implement WireType trait
    let expanded = match &input.data {
        Data::Struct(data_struct) => {
            let fields = match &data_struct.fields {
                Fields::Named(fields) => &fields.named,
                _ => {
                    return syn::Error::new_spanned(
                        &input,
                        "ffi_type only supports structs with named fields",
                    )
                    .to_compile_error()
                    .into();
                }
            };

            // Generate field info generation for each field
            let field_generations: Vec<_> = fields
                .iter()
                .map(|field| {
                    let field_name = field.ident.as_ref().unwrap();
                    let field_name_str = field_name.to_string();
                    let field_type = &field.ty;

                    quote! {
                        fields.push(oxidizer_core::FieldInfo::new(
                            #field_name_str,
                            offset,
                            std::mem::size_of::<#field_type>(),
                            <#field_type as oxidizer_core::WireType>::get_type_info(),
                        ));
                        offset += std::mem::size_of::<#field_type>();
                    }
                })
                .collect();

            let struct_name_str = struct_name.to_string();

            // Choose TypeKind based on whether this is a heap type
            let type_kind = if is_heap {
                quote! { oxidizer_core::TypeKind::HeapAllocated }
            } else {
                quote! { oxidizer_core::TypeKind::UserDefined }
            };

            // Note: For heap types, we don't generate per-type handles.
            // Users should use HeapAllocated<T> directly from rust_lib::heap_allocated.

            quote! {
                #[repr(C)]
                #vis struct #struct_name {
                    #fields
                }

                impl oxidizer_core::WireType for #struct_name {
                    fn get_type_info() -> oxidizer_core::TypeInfo {
                        let mut fields = Vec::new();
                        let mut offset = 0;

                        #(#field_generations)*

                        oxidizer_core::TypeInfo::new(
                            #struct_name_str,
                            std::mem::size_of::<Self>(),
                            fields,
                            #type_kind,
                        )
                    }
                }
            }
        }
        _ => {
            return syn::Error::new_spanned(&input, "ffi_type can only be applied to structs")
                .to_compile_error()
                .into();
        }
    };

    TokenStream::from(expanded)
}

/// A procedural macro to generate FFI function wrappers
///
/// Transforms a function like:
/// ```rust
/// #[ffi_function]
/// fn add(x: u64, y: u64) -> FFITy {
///     FFITy { x, y }
/// }
/// ```
///
/// Into:
/// ```rust
/// struct add;
///
/// impl add {
///     #[unsafe(export_name = "add")]
///     pub extern "C" fn call(x: u64, y: u64) -> FFITy {
///         FFITy { x, y }
///     }
/// }
///
/// impl WireFunction for add {
///     fn get_function_info() -> oxidizer_core::FunctionInfo {
///         oxidizer_core::FunctionInfo {
///             name: "add",
///             parameters: vec![u64::get_type_info(), u64::get_type_info()],
///             return_type: FFITy::get_type_info(),
///         }
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn ffi_function(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_name_str = fn_name.to_string();
    let fn_block = &input.block;
    let fn_inputs = &input.sig.inputs;
    let is_async = input.sig.asyncness.is_some();

    // Parse the attribute to get the runtime parameter
    let runtime_expr = if attr.is_empty() {
        // Default to Handle::current() if no runtime is provided
        quote! { tokio::runtime::Handle::current() }
    } else {
        // Parse the provided runtime expression and get its handle
        let runtime_tokens: proc_macro2::TokenStream = attr.into();
        quote! { #runtime_tokens.handle() }
    };

    // Extract parameter types for WireFunction implementation
    let param_types: Vec<_> = input
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                Some(&pat_type.ty)
            } else {
                None
            }
        })
        .collect();

    let param_names: Vec<_> = input
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    let ident = pat_ident.ident.to_string();
                    Some(ident)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // Extract return type for the call method and WireFunction implementation
    let (call_return_type, wire_return_type) = match &input.sig.output {
        syn::ReturnType::Type(arrow, ty) => (
            quote! { #arrow #ty },
            quote! { <#ty as oxidizer_core::WireType>::get_type_info() },
        ),
        syn::ReturnType::Default => (
            quote! {},
            quote! {
                oxidizer_core::TypeInfo::new(
                    "()",
                    0,
                    Vec::new(),
                    oxidizer_core::TypeKind::Void,
                )
            },
        ),
    };

    // Generate different implementations based on whether function is async
    let expanded = if is_async {
        // For async functions, transform according to README strategy
        let fn_name_internal = syn::Ident::new(&format!("{fn_name}_internal"), fn_name.span());

        // Extract the return type for callback
        let return_type = match &input.sig.output {
            syn::ReturnType::Type(_, ty) => quote! { #ty },
            syn::ReturnType::Default => quote! { () },
        };

        // Generate parameter names as expressions for the internal call
        let param_exprs: Vec<_> = input
            .sig
            .inputs
            .iter()
            .filter_map(|arg| {
                if let syn::FnArg::Typed(pat_type) = arg {
                    if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                        Some(&pat_ident.ident)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        // Build the parameter list for the exported C function
        let mut c_fn_params = vec![quote! { id: u64 }];
        for input in &input.sig.inputs {
            c_fn_params.push(quote! { #input });
        }
        c_fn_params.push(quote! { cb: extern "C" fn(u64, #return_type) });

        quote! {
            #[allow(non_camel_case_types)]
            struct #fn_name;

            impl #fn_name {
                // Internal async function with original logic
                async fn #fn_name_internal(#fn_inputs) #call_return_type {
                    #fn_block
                }

                // Exported C function that takes id and callback
                #[unsafe(export_name = #fn_name_str)]
                pub extern "C" fn call(#(#c_fn_params),*) {
                    // Use the provided runtime handle
                    let rt = #runtime_expr;

                    rt.spawn(async move {
                        let result = Self::#fn_name_internal(#(#param_exprs),*).await;
                        cb(id, result);
                    });
                }
            }

            impl oxidizer_core::WireFunction for #fn_name {
                fn get_function_info() -> oxidizer_core::FunctionInfo {
                    let mut parameters = vec![
                        // oxidizer_core::FunctionParameter::new("id", oxidizer_core::TypeInfo::new("u64", 8, vec![], oxidizer_core::TypeKind::U64)),
                    ];
                    #(parameters.push(oxidizer_core::FunctionParameter::new(#param_names, <#param_types as oxidizer_core::WireType>::get_type_info()));)*
                    // parameters.push(oxidizer_core::FunctionParameter::new("cb", oxidizer_core::TypeInfo::new("callback", 8, vec![], oxidizer_core::TypeKind::UserDefined)));

                    oxidizer_core::FunctionInfo::new(
                        #fn_name_str,
                        parameters,
                        #wire_return_type,
                        #is_async
                    )
                }
            }
        }
    } else {
        // For sync functions, keep original behavior
        quote! {
            #[allow(non_camel_case_types)]
            struct #fn_name;

            impl #fn_name {
                #[unsafe(export_name = #fn_name_str)]
                pub extern "C" fn call(#fn_inputs) #call_return_type {
                    #fn_block
                }
            }

            impl oxidizer_core::WireFunction for #fn_name {
                fn get_function_info() -> oxidizer_core::FunctionInfo {
                    oxidizer_core::FunctionInfo::new(
                        #fn_name_str,
                        vec![
                            #(oxidizer_core::FunctionParameter::new(#param_names, <#param_types as oxidizer_core::WireType>::get_type_info())),*
                        ],
                        #wire_return_type,
                        #is_async
                    )
                }
            }
        }
    };

    TokenStream::from(expanded)
}
