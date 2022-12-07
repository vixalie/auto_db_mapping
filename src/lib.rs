#![allow(dead_code)]

extern crate proc_macro;

use proc_macro::TokenStream;
use syn::DeriveInput;

mod attributes;
mod joined_row;
mod row;
mod types;

#[proc_macro_derive(AutoMapping, attributes(mapping))]
pub fn derive_from_mapping_row(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    match row::expand_derive_from_mapping_row(&input) {
        Ok(ts) => ts.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(JoinMapping)]
pub fn derive_from_joined_mapping_row(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    match joined_row::expand_derive_from_joined_mapping_row(&input) {
        Ok(ts) => ts.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
