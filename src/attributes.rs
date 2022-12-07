use proc_macro2::Ident;
use syn::{spanned::Spanned, Attribute, Lit, Meta, MetaList, MetaNameValue, NestedMeta};

use crate::types::{ChildAttributes, ContainerAttributes, RenameAll, TypeName};

macro_rules! fail {
    ($t:expr, $m:expr) => {
        return Err(syn::Error::new_spanned($t, $m))
    };
}

macro_rules! try_set {
    ($i:ident, $v:expr, $t:expr) => {
        match $i {
            None => $i = Some($v),
            Some(_) => fail!($t, "duplicate attribute"),
        }
    };
}

pub fn parse_container_attributes(input: &[Attribute]) -> syn::Result<ContainerAttributes> {
    let mut transparent: Option<bool> = None;
    let mut type_name: Option<TypeName> = None;
    let mut rename_all: Option<RenameAll> = None;
    let mut possible_prefix: Vec<String> = vec![];
    let mut possible_separator: Vec<String> = vec![];

    // 此处取得的是具备名为`mapping`派生属性的结构体中的所有结构体级的元标记，即具备`#[mapping]`
    for attr in input.iter().filter(|a| a.path.is_ident("mapping")) {
        // 将属性转换为元数据
        let meta = attr
            .parse_meta()
            .map_err(|e| syn::Error::new_spanned(attr, e))?;
        // 对已经获取到的结构体上的所有元标记
        match meta {
            // 此处匹配的内容是放置在独立的元标记中的，即`#[mapping()]`
            Meta::List(list) if list.path.is_ident("mapping") => {
                // `nested`中保存的是`mapping`属性标记列表，使用`NestedMeta`表示元属性中可能存在混合类型的内容
                for value in list.nested.iter() {
                    match value {
                        // 匹配以元属性形式出现的内容，`NestedMeta::Lit`表示以字面量形式出现的内容
                        NestedMeta::Meta(meta) => match meta {
                            // 匹配名为`transparent`的元属性，`transparent`只是一个键也就是`Path`类型，其是一个开关型的量
                            Meta::Path(p) if p.is_ident("transparent") => {
                                try_set!(transparent, true, value);
                            }
                            // 匹配键值对中键为`rename_all`的元属性，`MetaNameValue`中`path`为元属性的键，`lit`为元属性携带的值
                            Meta::NameValue(MetaNameValue {
                                path,
                                lit: Lit::Str(val),
                                ..
                            }) if path.is_ident("rename_all") => {
                                let val = match &*val.value() {
                                    "lowercase" => RenameAll::LowerCase,
                                    "snake_case" => RenameAll::SnakeCase,
                                    "UPPERCASE" => RenameAll::UpperCase,
                                    "SCREAMING_SNAKE_CASE" => RenameAll::ScreamingSnakeCase,
                                    "kebab-case" => RenameAll::KebabCase,
                                    "camelCase" => RenameAll::CamelCase,
                                    "PascalCase" => RenameAll::PascalCase,
                                    _ => fail!(meta, "unexpected value for rename_all"),
                                };
                                try_set!(rename_all, val, value);
                            }
                            // 匹配键值对中键为`type_name`的元属性
                            Meta::NameValue(MetaNameValue {
                                path,
                                lit: Lit::Str(val),
                                ..
                            }) if path.is_ident("type_name") => {
                                try_set!(
                                    type_name,
                                    TypeName {
                                        val: val.value(),
                                        span: value.span()
                                    },
                                    value
                                );
                            }
                            // 匹配内容列表为字符串字面量且名称为`possible_prefix`的元属性，即`possible_prefix("w", "x")`，非字面量内容会被丢弃。
                            Meta::List(MetaList { path, nested, .. })
                                if path.is_ident("possible_prefix") =>
                            {
                                let prefixes = nested.iter().filter_map(|m| match m {
                                    NestedMeta::Lit(Lit::Str(val)) => Some(val.value()),
                                    _ => None,
                                });
                                possible_prefix.extend(prefixes);
                            }
                            // 匹配内容列表为字符串字面量且名称为`possible_separator`的元属性，即`possible_separator("_", "__")`
                            Meta::List(MetaList { path, nested, .. })
                                if path.is_ident("possible_separator") =>
                            {
                                let separators = nested.iter().filter_map(|s| match s {
                                    NestedMeta::Lit(Lit::Str(val)) => Some(val.value()),
                                    _ => None,
                                });
                                possible_separator.extend(separators);
                            }
                            u => fail!(u, "unexpected mapping attribute"),
                        },
                        u => fail!(u, "unexpected mapping attribute"),
                    }
                }
            }
            _ => {}
        }
    }

    Ok(ContainerAttributes {
        transparent: transparent.unwrap_or(false),
        type_name,
        rename_all,
        possible_prefix,
        possible_separator,
    })
}

pub fn parse_child_attributes(input: &[Attribute]) -> syn::Result<ChildAttributes> {
    let mut alias: Option<String> = None;
    let mut default = false;
    let mut try_from: Option<Ident> = None;
    let mut flatten = false;

    // 与`parse_container_attributes()`中的获取元属性不同，这里获取的是属性上全部元标记名称为`mapping`的元标记
    for attr in input.iter().filter(|a| a.path.is_ident("mapping")) {
        let meta = attr
            .parse_meta()
            .map_err(|e| syn::Error::new_spanned(attr, e))?;

        // 获取`mapping`元标记中列出的所有元属性，即`#[mapping()]`中圆括号中的部分
        if let Meta::List(list) = meta {
            // 获取元属性列表中每一个元属性定义，并进行匹配
            for value in list.nested.iter() {
                match value {
                    NestedMeta::Meta(meta) => match meta {
                        Meta::NameValue(MetaNameValue {
                            path,
                            lit: Lit::Str(val),
                            ..
                        }) if path.is_ident("alias") => try_set!(alias, val.value(), value),
                        // 匹配键值对名为`try_from`的元属性，但是这里不使用`.value()`直接获取字面量，而是使用`.parse()`将其转换为表达式
                        Meta::NameValue(MetaNameValue {
                            path,
                            lit: Lit::Str(val),
                            ..
                        }) if path.is_ident("try_from") => try_set!(try_from, val.parse()?, value),
                        Meta::Path(path) if path.is_ident("default") => default = true,
                        Meta::Path(path) if path.is_ident("flatten") => flatten = true,
                        u => fail!(u, "unexpected mapping attribute"),
                    },
                    u => fail!(u, "unexpected mapping attribute"),
                }
            }
        }
    }

    Ok(ChildAttributes {
        alias,
        default,
        flatten,
        try_from,
    })
}
