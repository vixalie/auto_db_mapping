use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_quote, punctuated::Punctuated, token::Comma, Data, DataStruct, DeriveInput, Expr, Field,
    Fields, FieldsNamed, Stmt, Type, TypePath,
};

use crate::{
    attributes::{parse_child_attributes, parse_container_attributes},
    types::rename_all,
};

pub fn expand_derive_from_mapping_row(input: &DeriveInput) -> syn::Result<TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) => expand_derive_from_mapping_row_struct(input, named),
        _ => Err(syn::Error::new_spanned(input, "unsupported data structure")),
    }
}

fn is_path_option(typ: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = typ {
        path.leading_colon.is_none() && path.segments.iter().next().unwrap().ident == "Option"
    } else {
        false
    }
}

fn expand_derive_from_mapping_row_struct(
    input: &DeriveInput,
    fields: &Punctuated<Field, Comma>,
) -> syn::Result<TokenStream> {
    let ident = &input.ident;
    let container_attributes = parse_container_attributes(&input.attrs)?;

    let processes: Vec<Stmt> = fields
        .iter()
        .filter_map(|field| -> Option<Stmt> {
            let ident = &field.ident.as_ref()?;
            let attributes = parse_child_attributes(&field.attrs).unwrap();
            let ty = &field.ty;
            let is_option = is_path_option(ty);

            let expr: Expr = match (attributes.flatten, attributes.try_from) {
                (true, None) => {
                    parse_quote!(<#ty as ::sqlx::FromRow<'r, ::sqlx::postgres::PgRow>>::from_row(row))
                }
                (false, None) => {
                    let id_s = attributes
                        .alias
                        .or_else(|| Some(ident.to_string().trim_start_matches("r#").to_owned()))
                        .map(|s| match container_attributes.rename_all {
                            Some(pattern) => rename_all(&s, pattern),
                            None => s,
                        })
                        .unwrap();
                    let mut possibles = vec![];
                    for p in container_attributes.possible_prefix.iter() {
                        for s in container_attributes.possible_separator.iter() {
                            possibles.push(format!("{}{}{}", p, s, id_s));
                        }
                    }
                    possibles.push(id_s.clone());
                    let possible_columns = possibles.as_slice();
                    parse_quote!({
                        let index = [#(#possible_columns),*].iter().find(|col| row.try_column(&col).is_ok());
                        match (index, #is_option) {
                            (Some(index), true) => match row.try_get(index) {
                                Ok(cell) => ::sqlx::Result::Ok(Some(cell)),
                                Err(err) => match err {
                                    ::sqlx::Error::ColumnNotFound(_) => ::sqlx::Result::Ok(None),
                                    _ => ::sqlx::Result::Err(err),
                                }
                            },
                            (None, true) => ::sqlx::Result::Ok(None),
                            (Some(index), false) => row.try_get(index),
                            (None, false) => ::sqlx::Result::Err(::sqlx::Error::ColumnNotFound("[auto_mapping]FromRow: try_get failed".to_string()))
                        }
                    })
                },
                (true, Some(try_from)) => {
                    parse_quote!(
                        <#try_from as ::sqlx::FromRow<'r, ::sqlx::postgres::PgRow>>::from_row(row)
                            .and_then(|v| <#ty as ::std::convert::TryFrom::<#try_from>>::try_from(v)
                                .map_err(|e| ::sqlx::Error::ColumnNotFound("[auto_mapping]FromRow: try_from failed",to_string()))))
                },
                (false, Some(try_from)) => {
                    let id_s = attributes
                        .alias
                        .or_else(|| Some(ident.to_string().trim_start_matches("r#").to_owned()))
                        .map(|s| match container_attributes.rename_all {
                            Some(pattern) => rename_all(&s, pattern),
                            None => s,
                        })
                        .unwrap();
                    let mut possibles = vec![];
                    for p in container_attributes.possible_prefix.iter() {
                        for s in container_attributes.possible_separator.iter() {
                            possibles.push(format!("{}{}{}", p, s, id_s));
                        }
                    }
                    possibles.push(id_s.clone());
                    let possible_columns = possibles.as_slice();
                    parse_quote!({
                        let index = [#(#possible_columns),*].iter().find(|col| row.try_column(&col).is_ok());
                        match (index, #is_option) {
                            (Some(index), true) => match row.try_get(index).and_then(|v| <#ty as ::std::convert::TryFrom::<#try_from>>::try_from(v).map_err(|e| ::sqlx::Error::ColumnNotFound("[auto_mapping]FromRow: try_from failed".to_string()))) {
                                Ok(cell) => ::sqlx::Result::Ok(cell),
                                Err(err) => match err {
                                    ::sqlx::Error::ColumnNotFound(_) => ::sqlx::Result::Ok(None),
                                    _ => ::sqlx::Result::Err(err),
                                }
                            },
                            (None, true) => ::sqlx::Result::Ok(None),
                            (Some(index), false) => row.try_get(index).and_then(|v| <#ty as ::std::convert::TryFrom::<#try_from>>::try_from(v).map_err(|e| ::sqlx::Error::ColumnNotFound("[auto_mapping]FromRow: try_from failed".to_string()))),
                            (None, false) => ::sqlx::Result::Err(::sqlx::Error::ColumnNotFound("[auto_mapping]FromRow: try_from failed".to_string()))
                        }
                    })
                },
            };

            if attributes.default {
                Some(parse_quote!(
                    let #ident: #ty = #expr.or_else(|e| match e {
                        ::sqlx::Error::ColumnNotFound(_) => {
                            ::sqlx::Result::Ok(Default::default())
                        },
                        e => ::sqlx::Result::Err(e),
                    })?.unwrap();
                ))
            } else {
                Some(parse_quote!(
                    let #ident: #ty = #expr?.unwrap();
                ))
            }
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
