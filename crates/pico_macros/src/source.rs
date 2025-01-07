use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Error, Fields};

pub(crate) fn source(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = input.ident.clone();

    let fields = match input.data {
        Data::Struct(ref data) => match &data.fields {
            Fields::Named(fields) => fields.named.clone(),
            _ => {
                return Error::new_spanned(&data.fields, "expected named fields")
                    .to_compile_error()
                    .into()
            }
        },
        _ => {
            return Error::new_spanned(&input, "expected a struct")
                .to_compile_error()
                .into()
        }
    };

    let key_field_name = fields
        .iter()
        .find(|field| {
            field.attrs.iter().any(|attr| {
                attr.path()
                    .segments
                    .last()
                    .map_or(false, |segment| segment.ident == "key")
            })
        })
        .and_then(|field| field.ident.clone());

    let field_name = match key_field_name {
        Some(field_name) => field_name,
        None => {
            return Error::new_spanned(
                &struct_name,
                "#[key] attribute must be set on a struct field",
            )
            .to_compile_error()
            .into();
        }
    };

    let output = quote! {
        impl ::pico_core::source::Source for #struct_name {
            fn get_key(&self) -> ::pico_core::source::SourceKey {
                ::pico_core::source::SourceKey::intern(&self.#field_name)
            }
        }
    };

    output.into()
}
