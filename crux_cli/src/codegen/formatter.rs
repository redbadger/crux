use std::collections::BTreeMap;

use ascent::ascent;
use rustdoc_types::{GenericArg, GenericArgs, Item, ItemEnum, Type};

use crate::codegen::collect;

use super::{
    indexed::Indexed,
    item::*,
    node::ItemNode,
    serde::case::RenameRule,
    serde_generate::format::{ContainerFormat, Format, Named, VariantFormat},
};

ascent! {
    #![measure_rule_times]
    pub struct Formatter;

    // ------- facts ------------------
    relation edge(ItemNode, ItemNode);

    // ------- rules ------------------

    relation struct_unit(ItemNode);
    struct_unit(s) <-- edge(s, _), if is_struct_unit(&s.item);

    relation struct_plain(ItemNode);
    struct_plain(s) <-- edge(s, _), if is_struct_plain(&s.item);

    relation struct_tuple(ItemNode);
    struct_tuple(s) <-- edge(s, _), if is_struct_tuple(&s.item);

    relation field(ItemNode, ItemNode);
    field(x, f) <-- edge(x, f), if x.has_field(f);

    relation fields(ItemNode, Vec<ItemNode>);
    fields(x, fields) <--
        field(x, f),
        agg fs = collect(f) in field(x, f),
        let fields = x.fields(fs);

    relation variant(ItemNode, ItemNode);
    variant(e, v) <-- edge(e, v), if e.has_variant(v);

    relation variants(ItemNode, Vec<ItemNode>);
    variants(e, variants) <--
        variant(e, v),
        agg vs = collect(v) in variant(e, v),
        let variants = e.variants(vs);

    relation variant_plain(ItemNode, ItemNode);
    variant_plain(e, v) <-- variant(e, v), if is_plain_variant(&v.item);

    relation variant_tuple(ItemNode, ItemNode);
    variant_tuple(e, v) <-- variant(e, v), if is_tuple_variant(&v.item);

    relation variant_struct(ItemNode, ItemNode);
    variant_struct(e, v) <-- variant(e, v), if is_struct_variant(&v.item);

    relation format(ItemNode, Indexed<Format>);
    format(x, format) <--
        field(x, field),
        fields(x, fields),
        if let Some(format) = make_format(field, fields);

    relation format_named(ItemNode, Indexed<Named<Format>>);
    format_named(x, format) <--
        field(x, field),
        fields(x, fields),
        if let Some(format) = make_named_format(field, fields, x);

    relation format_plain_variant(ItemNode, Indexed<Named<VariantFormat>>);
    format_plain_variant(e, format) <--
        variant_plain(e, v),
        variants(e, variants),
        if let Some(format) = make_plain_variant_format(v, variants, e);

    relation format_tuple_variant(ItemNode, Indexed<Named<VariantFormat>>);
    format_tuple_variant(e, format) <--
        variant_tuple(e, v),
        variants(e, variants),
        agg formats = collect(format) in format(v, format),
        if let Some(format) = make_tuple_variant_format(v, &formats, variants, e);

    relation format_struct_variant(ItemNode, Indexed<Named<VariantFormat>>);
    format_struct_variant(e, format) <--
        variant_struct(e, v),
        variants(e, variants),
        agg formats = collect(format) in format_named(v, format),
        if let Some(format) = make_struct_variant_format(v, &formats, variants, e);

    relation format_variant(ItemNode, Indexed<Named<VariantFormat>>);
    format_variant(e, format) <-- format_plain_variant(e, format);
    format_variant(e, format) <-- format_tuple_variant(e, format);
    format_variant(e, format) <-- format_struct_variant(e, format);

    relation container(String, ContainerFormat);
    container(name, container) <--
        struct_plain(s),
        if let Some(name) = s.name(),
        agg field_formats = collect(format) in format_named(s, format),
        let container = make_struct_plain(&field_formats);
    container(name, container) <--
        struct_unit(s),
        if let Some(name) = s.name(),
        let container = make_struct_unit();
    container(name, container) <--
        struct_tuple(s),
        if let Some(name) = s.name(),
        agg field_formats = collect(format) in format(s, format),
        let container = make_struct_tuple(&field_formats);
    container(name, container) <--
        variant(e, _),
        if let Some(name) = e.name(),
        agg variant_formats = collect(format) in format_variant(e, format),
        let container = make_enum(&variant_formats);
    container("Range".to_string(), container) <--
        field(_, f), if f.is_range(),
        if let Some(container) = make_range(f);
    container("Request".to_string(), container) <--
        let container = make_request();
}

fn make_format(field: &ItemNode, all_fields: &[ItemNode]) -> Option<Indexed<Format>> {
    let index = all_fields.iter().position(|f| f == field)?;
    match &field.item.inner {
        ItemEnum::StructField(type_) => Some(Indexed {
            index: index as u32,
            value: {
                if let Some((_whole, serde_with)) = field.item.attrs.iter().find_map(|attr| {
                    lazy_regex::regex_captures!(r#"\[serde\(with\s*=\s*"(\w+)"\)\]"#, attr)
                }) {
                    match serde_with {
                        "serde_bytes" => Format::Bytes, // e.g. HttpRequest.body, HttpResponse.body
                        _ => todo!(),
                    }
                } else {
                    type_.into()
                }
            },
        }),
        _ => None,
    }
}

fn make_named_format(
    field: &ItemNode,
    all_fields: &[ItemNode],
    struct_: &ItemNode,
) -> Option<Indexed<Named<Format>>> {
    match field.name() {
        Some(name) => match make_format(field, all_fields) {
            Some(Indexed { index, value }) => Some(Indexed {
                index,
                value: Named {
                    name: field_name(name, &field.item.attrs, &struct_.item.attrs),
                    value,
                },
            }),
            _ => None,
        },
        _ => None,
    }
}

fn make_plain_variant_format(
    variant: &ItemNode,
    all_variants: &[ItemNode],
    enum_: &ItemNode,
) -> Option<Indexed<Named<VariantFormat>>> {
    let index = all_variants.iter().position(|f| f == variant)?;
    match &variant.item {
        Item {
            name: Some(name),
            inner: ItemEnum::Variant(_),
            ..
        } => Some(Indexed {
            index: index as u32,
            value: Named {
                name: variant_name(name, &variant.item.attrs, &enum_.item.attrs),
                value: VariantFormat::Unit,
            },
        }),
        _ => None,
    }
}

fn make_struct_variant_format(
    variant: &ItemNode,
    fields: &[(&Indexed<Named<Format>>,)],
    all_variants: &[ItemNode],
    enum_: &ItemNode,
) -> Option<Indexed<Named<VariantFormat>>> {
    let index = all_variants.iter().position(|f| f == variant)?;
    match &variant.item {
        Item {
            name: Some(name),
            inner: ItemEnum::Variant(_),
            ..
        } => {
            let mut fields = fields.to_owned();
            fields.sort();
            let fields = fields.iter().map(|(f,)| f.inner()).collect::<Vec<_>>();
            Some(Indexed {
                index: index as u32,
                value: Named {
                    name: variant_name(name, &variant.item.attrs, &enum_.item.attrs),
                    value: VariantFormat::Struct(fields),
                },
            })
        }
        _ => None,
    }
}

fn make_tuple_variant_format(
    variant: &ItemNode,
    fields: &[(&Indexed<Format>,)],
    all_variants: &[ItemNode],
    enum_: &ItemNode,
) -> Option<Indexed<Named<VariantFormat>>> {
    let index = all_variants.iter().position(|v| v == variant)?;
    match &variant.item {
        Item {
            name: Some(name),
            inner: ItemEnum::Variant(_),
            ..
        } => {
            let mut fields = fields.to_owned();
            fields.sort();
            let fields = fields.iter().map(|(f,)| f.inner()).collect::<Vec<_>>();
            let value = match fields.len() {
                0 => VariantFormat::Unit,
                1 => VariantFormat::NewType(Box::new(fields[0].clone())),
                _ => VariantFormat::Tuple(fields),
            };
            Some(Indexed {
                index: index as u32,
                value: Named {
                    name: variant_name(name, &variant.item.attrs, &enum_.item.attrs),
                    value,
                },
            })
        }
        _ => None,
    }
}

fn make_struct_unit() -> ContainerFormat {
    ContainerFormat::UnitStruct
}

fn make_struct_plain(fields: &[(&Indexed<Named<Format>>,)]) -> ContainerFormat {
    let mut fields = fields.to_owned();
    fields.sort();
    let fields = fields.iter().map(|(f,)| f.inner()).collect::<Vec<_>>();
    match fields.len() {
        0 => ContainerFormat::UnitStruct,
        _ => ContainerFormat::Struct(fields),
    }
}

fn make_struct_tuple(fields: &[(&Indexed<Format>,)]) -> ContainerFormat {
    let mut fields = fields.to_owned();
    fields.sort();
    let fields = fields.iter().map(|(f,)| f.inner()).collect::<Vec<_>>();
    match fields.len() {
        0 => ContainerFormat::UnitStruct,
        1 => ContainerFormat::NewTypeStruct(Box::new(fields[0].clone())),
        _ => ContainerFormat::TupleStruct(fields),
    }
}

fn make_enum(formats: &[(&Indexed<Named<VariantFormat>>,)]) -> ContainerFormat {
    let mut map = BTreeMap::default();
    for (Indexed { index, value },) in formats {
        map.insert(*index, value.clone());
    }
    ContainerFormat::Enum(map)
}

fn make_range(field: &ItemNode) -> Option<ContainerFormat> {
    match &field.item.inner {
        ItemEnum::StructField(range_type) => {
            let field_format: Option<Format> = match range_type {
                Type::ResolvedPath(path) => match &path.args {
                    Some(args) => match args.as_ref() {
                        GenericArgs::AngleBracketed { args, .. } => {
                            let type_ = args.iter().next()?;
                            match type_ {
                                GenericArg::Type(ref type_) => Some(type_.into()),
                                _ => None,
                            }
                        }
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            };
            field_format.map(|f| {
                ContainerFormat::Struct(vec![
                    Named {
                        name: "start".to_string(),
                        value: f.clone(),
                    },
                    Named {
                        name: "end".to_string(),
                        value: f.clone(),
                    },
                ])
            })
        }
        _ => None,
    }
}

fn make_request() -> ContainerFormat {
    ContainerFormat::Struct(vec![
        Named {
            name: "id".to_string(),
            value: Format::U32,
        },
        Named {
            name: "effect".to_string(),
            value: Format::TypeName("Effect".to_string()),
        },
    ])
}

impl From<&Type> for Format {
    fn from(type_: &Type) -> Self {
        match type_ {
            Type::ResolvedPath(path) => {
                let name = path_to_string(path);
                if let Some(args) = &path.args {
                    match args.as_ref() {
                        GenericArgs::AngleBracketed {
                            args,
                            constraints: _,
                        } => match name.as_str() {
                            "Option" => {
                                let format = match args[0] {
                                    GenericArg::Type(ref type_) => type_.into(),
                                    _ => todo!(),
                                };
                                Format::Option(Box::new(format))
                            }
                            "String" => Format::Str,
                            "Vec" => {
                                let format = match args[0] {
                                    GenericArg::Type(ref type_) => type_.into(),
                                    _ => todo!(),
                                };
                                Format::Seq(Box::new(format))
                            }
                            _ => Format::TypeName(name),
                        },
                        GenericArgs::Parenthesized {
                            inputs: _,
                            output: _,
                        } => todo!(),
                    }
                } else {
                    Format::TypeName(name)
                }
            }
            Type::DynTrait(_dyn_trait) => todo!(),
            Type::Generic(_param_name) => todo!(),
            Type::Primitive(s) => match s.as_ref() {
                "bool" => Format::Bool,
                "char" => Format::Char,
                "isize" => match std::mem::size_of::<isize>() {
                    4 => Format::I32,
                    8 => Format::I64,
                    _ => panic!("unsupported isize size"),
                },
                "i8" => Format::I8,
                "i16" => Format::I16,
                "i32" => Format::I32,
                "i64" => Format::I64,
                "i128" => Format::I128,
                "usize" => match std::mem::size_of::<usize>() {
                    4 => Format::U32,
                    8 => Format::U64,
                    _ => panic!("unsupported usize size"),
                },
                "u8" => Format::U8,
                "u16" => Format::U16,
                "u32" => Format::U32,
                "u64" => Format::U64,
                "u128" => Format::U128,
                s => panic!("need to implement primitive {s}"),
            },
            Type::FunctionPointer(_function_pointer) => todo!(),
            Type::Tuple(vec) => Format::Tuple(vec.iter().map(|t| t.into()).collect()),
            Type::Slice(_) => todo!(),
            Type::Array { type_: _, len: _ } => todo!(),
            Type::Pat {
                type_: _,
                __pat_unstable_do_not_use,
            } => todo!(),
            Type::ImplTrait(_vec) => todo!(),
            Type::Infer => todo!(),
            Type::RawPointer {
                is_mutable: _,
                type_: _,
            } => todo!(),
            Type::BorrowedRef {
                lifetime: _,
                is_mutable: _,
                type_: _,
            } => todo!(),
            Type::QualifiedPath {
                name,
                args: _,
                self_type: _,
                trait_: _,
            } => Format::TypeName(name.to_string()),
        }
    }
}

fn path_to_string(path: &rustdoc_types::Path) -> String {
    if let Some((_mod, name)) = path.path.rsplit_once("::") {
        name.to_string()
    } else {
        path.path.clone()
    }
}

fn variant_name<T>(name: &str, variant_attrs: &[T], enum_attrs: &[T]) -> String
where
    T: AsRef<str>,
{
    if let Some((_whole, rename)) = variant_attrs.iter().find_map(|attr| {
        lazy_regex::regex_captures!(r#"\[serde\(rename\s*=\s*"(\w+)"\)\]"#, attr.as_ref())
    }) {
        return rename.to_string();
    }

    if let Some((_whole, rename_all)) = enum_attrs.iter().find_map(|attr| {
        lazy_regex::regex_captures!(r#"\[serde\(rename_all\s*=\s*"(\w+)"\)\]"#, attr.as_ref())
    }) {
        return RenameRule::from_str(rename_all)
            .unwrap_or(RenameRule::None)
            .apply_to_variant(name);
    }

    name.to_string()
}

fn field_name<T>(name: &str, field_attrs: &[T], struct_attrs: &[T]) -> String
where
    T: AsRef<str>,
{
    if let Some((_whole, rename)) = field_attrs.iter().find_map(|attr| {
        lazy_regex::regex_captures!(r#"\[serde\(rename\s*=\s*"(\w+)"\)\]"#, attr.as_ref())
    }) {
        return rename.to_string();
    }

    if let Some((_whole, rename_all)) = struct_attrs.iter().find_map(|attr| {
        lazy_regex::regex_captures!(r#"\[serde\(rename_all\s*=\s*"(\w+)"\)\]"#, attr.as_ref())
    }) {
        return RenameRule::from_str(rename_all)
            .unwrap_or(RenameRule::None)
            .apply_to_field(name);
    }

    name.to_string()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("foo", &[""], &[], "foo")]
    #[case("foo", &["#[serde(rename = \"bar\")]"], &[], "bar")]
    #[case("FooBar", &[], &["#[serde(rename_all = \"camelCase\")]"], "fooBar")]
    #[case("FooBar", &[""], &["#[serde(with = \"something\")]",
        "#[serde(rename_all = \"snake_case\")]"], "foo_bar")]
    #[case("FooBar", &["#[serde(rename = \"bar\")]"], &["#[serde(with = \"something\")]",
        "#[serde(rename_all = \"snake_case\")]"], "bar")]
    fn variant_renaming<T: AsRef<str>>(
        #[case] name: &str,
        #[case] variant_attrs: &[T],
        #[case] enum_attrs: &[T],
        #[case] expected: String,
    ) {
        assert_eq!(variant_name(name, variant_attrs, enum_attrs), expected);
    }

    #[rstest]
    #[case("foo", &[""], &[], "foo")]
    #[case("foo", &["#[serde(rename = \"bar\")]"], &[], "bar")]
    #[case("foo_bar", &[], &["#[serde(rename_all = \"camelCase\")]"], "fooBar")]
    #[case("foo_bar", &[""], &["#[serde(with = \"something\")]",
        "#[serde(rename_all = \"PascalCase\")]"], "FooBar")]
    #[case("foo_bar", &["#[serde(rename = \"bar\")]"], &["#[serde(with = \"something\")]",
        "#[serde(rename_all = \"PascalCase\")]"], "bar")]
    fn field_renaming<T: AsRef<str>>(
        #[case] name: &str,
        #[case] field_attrs: &[T],
        #[case] struct_attrs: &[T],
        #[case] expected: String,
    ) {
        assert_eq!(field_name(name, field_attrs, struct_attrs), expected);
    }
}
