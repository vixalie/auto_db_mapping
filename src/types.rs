use heck::{ToKebabCase, ToLowerCamelCase, ToShoutyKebabCase, ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Ident, Span};

#[derive(Debug, Clone)]
pub struct TypeName {
    pub val: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ContainerAttributes {
    pub transparent: bool,
    pub type_name: Option<TypeName>,
    pub rename_all: Option<RenameAll>,
    pub possible_prefix: Vec<String>,
    pub possible_separator: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ChildAttributes {
    pub alias: Option<String>,
    pub default: bool,
    pub flatten: bool,
    pub try_from: Option<Ident>,
}

#[derive(Debug, Copy, Clone)]
pub enum RenameAll {
    LowerCase,
    SnakeCase,
    UpperCase,
    ScreamingSnakeCase,
    KebabCase,
    CamelCase,
    PascalCase,
}

pub fn rename_all(s: &str, pattern: RenameAll) -> String {
    match pattern {
        RenameAll::LowerCase => s.to_lowercase(),
        RenameAll::SnakeCase => s.to_snake_case(),
        RenameAll::UpperCase => s.to_uppercase(),
        RenameAll::ScreamingSnakeCase => s.to_shouty_kebab_case(),
        RenameAll::KebabCase => s.to_kebab_case(),
        RenameAll::CamelCase => s.to_lower_camel_case(),
        RenameAll::PascalCase => s.to_upper_camel_case(),
    }
}
