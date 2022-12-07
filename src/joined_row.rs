use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_quote, punctuated::Punctuated, token::Comma, Data, DataStruct, DeriveInput, Expr, Field,
    Fields, FieldsNamed, Stmt,
};

pub fn expand_derive_from_joined_mapping_row(input: &DeriveInput) -> syn::Result<TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) => expand_derive_from_mapping_row_struct(input, named),
        _ => Err(syn::Error::new_spanned(input, "unsupported data structure")),
    }
}

fn expand_derive_from_mapping_row_struct(
    input: &DeriveInput,
    fields: &Punctuated<Field, Comma>,
) -> syn::Result<TokenStream> {
    let ident = &input.ident;

    let processes: Vec<Stmt> = fields
        .iter()
        .filter_map(|field| -> Option<Stmt> {
            let ident = &field.ident.as_ref()?;
            let ty = &field.ty;

            let expr: Expr =
                parse_quote!(<#ty as ::sqlx::FromRow<'r, ::sqlx::postgres::PgRow>>::from_row(row));

            Some(parse_quote!(
                let #ident: #ty = #expr?;
            ))
        })
        .collect();

    let names = fields.iter().map(|f| &f.ident);

    Ok(quote!(
        #[automatically_derived]
        impl<'r> ::sqlx::FromRow<'r, ::sqlx::postgres::PgRow> for #ident {
            fn from_row(row: &'r ::sqlx::postgres::PgRow) -> ::sqlx::Result<Self> {
                #(#processes)*

                ::sqlx::Result::Ok(Self {
                    #(#names),*
                })
            }
        }
    ))
}
