use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Block, DeriveInput, Error};

extern crate proc_macro;

#[proc_macro_derive(Resolve)]
pub fn derive(input: TokenStream) -> TokenStream {
    let derive_input: DeriveInput = syn::parse(input).unwrap();

    let name = &derive_input.ident;
    let (impl_generics, ty_generics, where_clause) = derive_input.generics.split_for_impl();

    quote::quote! {
        impl #impl_generics rocket_dependency_injection::Resolve for #name #ty_generics #where_clause {
            fn resolve(service_provider: &rocket_dependency_injection::ServiceProvider) -> Self {
                Self::resolve_generated(service_provider)
            }
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn resolve_constructor(_: TokenStream, input: TokenStream) -> TokenStream {
    let function: syn::ItemFn = syn::parse(input).unwrap();

    validate_function(&function).unwrap();

    let resolve_generated = generate_resolve_generated(&function.sig);

    quote! {
        #function

        #resolve_generated
    }
    .into()
}

fn generate_resolve_generated(sig: &syn::Signature) -> proc_macro2::TokenStream {
    let fn_name = sig.ident.clone();
    let calls: Vec<proc_macro2::TokenStream> = sig
        .inputs
        .iter()
        .map(|_| quote!(service_provider.unwrap()))
        .collect();

    quote! {
        #[allow(unused_variables)]
        pub fn resolve_generated(service_provider: &rocket_dependency_injection::ServiceProvider) -> Self {
            Self::#fn_name(#(#calls),*)
        }
    }
}

fn validate_function(function: &syn::ItemFn) -> syn::Result<()> {
    validate_sig(&function.sig)?;
    validate_block(&function.block)?;
    Ok(())
}

fn validate_block(block: &Block) -> syn::Result<()> {
    match block.stmts.len() {
        0 => Err(Error::new(
            Span::call_site(),
            "resolve_constructor cannot be set on a function with empty block",
        )),
        _ => Ok(()),
    }
}

fn validate_sig(sig: &syn::Signature) -> syn::Result<()> {
    validate_output(&sig.output)?;
    validate_arguments(&sig.inputs)?;
    if sig.asyncness.is_some() {
        Err(Error::new(
            Span::call_site(),
            "resolve_constructor cannot be set on an async function",
        ))
    } else if sig.constness.is_some() {
        Err(Error::new(
            Span::call_site(),
            "resolve_constructor cannot be set on a const function",
        ))
    } else {
        Ok(())
    }
}

fn validate_arguments(
    inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
) -> syn::Result<()> {
    inputs.iter().all(|arg| match arg {
        syn::FnArg::Typed(typ) if !is_self_type(&typ.ty) => true,
        _ => false
    }).then_some(()).ok_or(Error::new(Span::call_site(), "resolve_constructor cannot be set on a function that have an argument of type Self (either self or Self)"))
}

fn validate_output(output: &syn::ReturnType) -> syn::Result<()> {
    match output {
        syn::ReturnType::Type(_, typ) if is_self_type(typ) => Ok(()),
        _ => Err(Error::new(
            Span::call_site(),
            "resolve_constructor cannot be set on a function that does not return Self",
        )),
    }
}

fn is_self_type(typ: &Box<syn::Type>) -> bool {
    match &**typ {
        syn::Type::Path(path) => match path.path.segments.len() {
            1 => match path
                .path
                .segments
                .last()
                .unwrap()
                .ident
                .to_string()
                .as_str()
            {
                "Self" => true,
                _ => false,
            },
            _ => false,
        },
        _ => false,
    }
}
