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
#[proc_macro_attribute]
pub fn ffi_type(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

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
                        fields.push(oxidize_core::FieldInfo {
                            name: #field_name_str,
                            offset: offset,
                            size: std::mem::size_of::<#field_type>(),
                            ty: <#field_type as oxidize_core::WireType>::get_type_info(),
                        });
                        offset += std::mem::size_of::<#field_type>();
                    }
                })
                .collect();

            let struct_name_str = struct_name.to_string();

            quote! {
                #[repr(C)]
                #vis struct #struct_name {
                    #fields
                }

                impl oxidize_core::WireType for #struct_name {
                    fn get_type_info() -> oxidize_core::TypeInfo {
                        let mut fields = Vec::new();
                        let mut offset = 0;

                        #(#field_generations)*

                        oxidize_core::TypeInfo {
                            name: #struct_name_str,
                            size: std::mem::size_of::<Self>(),
                            fields,
                            kind: oxidize_core::TypeKind::UserDefined,
                        }
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
///     fn get_function_signature() -> oxidize_core::FunctionSignature {
///         oxidize_core::FunctionSignature {
///             name: "add",
///             parameters: vec![u64::get_type_info(), u64::get_type_info()],
///             return_type: FFITy::get_type_info(),
///         }
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn ffi_function(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_name_str = fn_name.to_string();
    let fn_block = &input.block;
    let fn_inputs = &input.sig.inputs;

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
            quote! { <#ty as oxidize_core::WireType>::get_type_info() },
        ),
        syn::ReturnType::Default => (
            quote! {},
            quote! {
                oxidize_core::TypeInfo {
                    name: "()",
                    size: 0,
                    fields: Vec::new(),
                }
            },
        ),
    };

    let expanded = quote! {
        // non_camel_case_types

        #[allow(non_camel_case_types)]
        struct #fn_name;

        impl #fn_name {
            #[unsafe(export_name = #fn_name_str)]
            pub extern "C" fn call(#fn_inputs) #call_return_type {
                #fn_block
            }
        }

        impl oxidize_core::WireFunction for #fn_name {
            fn get_function_signature() -> oxidize_core::FunctionSignature {
                oxidize_core::FunctionSignature {
                    name: #fn_name_str,
                    parameters: vec![
                        #(oxidize_core::FunctionParameter::new(#param_names, <#param_types as oxidize_core::WireType>::get_type_info())),*
                    ],
                    return_type: #wire_return_type,
                }
            }
        }
    };

    TokenStream::from(expanded)
}
