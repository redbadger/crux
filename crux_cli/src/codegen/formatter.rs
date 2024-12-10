use std::collections::BTreeMap;

use ascent::ascent;
use rustdoc_types::{GenericArg, GenericArgs, Item, ItemEnum, Type, Variant, VariantKind};

use crate::codegen::node::collect;

use super::{
    indexed::Indexed,
    node::ItemNode,
    serde_generate::format::{ContainerFormat, Format, Named, VariantFormat},
};

ascent! {
    pub struct Formatter;

    // ------- facts ------------------
    relation edge(ItemNode, ItemNode);

    // ------- rules ------------------

    relation struct_unit(ItemNode);
    struct_unit(s) <-- edge(s, _), if s.is_struct_unit();

    relation struct_plain(ItemNode);
    struct_plain(s) <-- edge(s, _), if s.is_struct_plain();

    relation struct_tuple(ItemNode);
    struct_tuple(s) <-- edge(s, _), if s.is_struct_tuple();

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
    variant_plain(e, v) <-- variant(e, v), if is_plain_variant(&v);

    relation variant_tuple(ItemNode, ItemNode);
    variant_tuple(e, v) <-- variant(e, v), if is_tuple_variant(&v);

    relation variant_struct(ItemNode, ItemNode);
    variant_struct(e, v) <-- variant(e, v), if is_struct_variant(&v);

    relation format(ItemNode, Indexed<Format>);
    format(x, format) <--
        field(x, field),
        fields(x, fields),
        if let Some(format) = make_format(field, fields);

    relation format_named(ItemNode, Indexed<Named<Format>>);
    format_named(x, format) <--
        field(x, field),
        fields(x, fields),
        if let Some(format) = make_named_format(field, fields);

    relation format_plain_variant(ItemNode, Indexed<Named<VariantFormat>>);
    format_plain_variant(e, format) <--
        variant_plain(e, v),
        variants(e, variants),
        if let Some(format) = make_plain_variant_format(v, variants);

    relation format_tuple_variant(ItemNode, Indexed<Named<VariantFormat>>);
    format_tuple_variant(e, format) <--
        variant_tuple(e, v),
        variants(e, variants),
        agg formats = collect(format) in format(v, format),
        if let Some(format) = make_tuple_variant_format(v, &formats, variants);

    relation format_struct_variant(ItemNode, Indexed<Named<VariantFormat>>);
    format_struct_variant(e, format) <--
        variant_struct(e, v),
        variants(e, variants),
        agg formats = collect(format) in format_named(v, format),
        if let Some(format) = make_struct_variant_format(v, &formats, variants);

    relation format_variant(ItemNode, Indexed<Named<VariantFormat>>);
    format_variant(e, format) <-- format_plain_variant(e, format);
    format_variant(e, format) <-- format_tuple_variant(e, format);
    format_variant(e, format) <-- format_struct_variant(e, format);

    relation container(String, ContainerFormat);
    container(name, container) <--
        struct_plain(s),
        agg fields = collect(format) in format_named(s, format),
        if let Some(name) = s.name(),
        let container = make_struct_plain(&fields);
    container(name, container) <--
        struct_unit(s),
        if let Some(name) = s.name(),
        let container = make_struct_unit();
    container(name, container) <--
        struct_tuple(s),
        agg fields = collect(format) in format(s, format),
        if let Some(name) = s.name(),
        let container = make_struct_tuple(&fields);
    container(name, container) <--
        variant(e, _),
        agg variants = collect(format) in format_variant(e, format),
        if let Some(name) = e.name(),
        let container = make_enum(&variants);
}

fn make_format(field: &ItemNode, all_fields: &Vec<ItemNode>) -> Option<Indexed<Format>> {
    let index = all_fields.iter().position(|f| f == field)?;
    match &field.0.inner {
        ItemEnum::StructField(type_) => Some(Indexed {
            index: index as u32,
            value: type_.into(),
        }),
        _ => None,
    }
}

fn make_named_format(
    field: &ItemNode,
    all_fields: &Vec<ItemNode>,
) -> Option<Indexed<Named<Format>>> {
    match field.name() {
        Some(name) => match make_format(field, all_fields) {
            Some(Indexed { index, value }) => Some(Indexed {
                index,
                value: Named {
                    name: name.to_string(),
                    value,
                },
            }),
            _ => None,
        },
        _ => None,
    }
}

fn is_plain_variant(variant: &ItemNode) -> bool {
    matches!(
        &variant.0,
        Item {
            inner: ItemEnum::Variant(Variant {
                kind: VariantKind::Plain,
                ..
            }),
            ..
        }
    )
}

fn is_struct_variant(variant: &ItemNode) -> bool {
    matches!(
        &variant.0,
        Item {
            inner: ItemEnum::Variant(Variant {
                kind: VariantKind::Struct { .. },
                ..
            }),
            ..
        }
    )
}

fn is_tuple_variant(variant: &ItemNode) -> bool {
    matches!(
        &variant.0,
        Item {
            inner: ItemEnum::Variant(Variant {
                kind: VariantKind::Tuple(_),
                ..
            }),
            ..
        }
    )
}

fn make_plain_variant_format(
    variant: &ItemNode,
    all_variants: &Vec<ItemNode>,
) -> Option<Indexed<Named<VariantFormat>>> {
    let index = all_variants.iter().position(|f| f == variant)?;
    match &variant.0 {
        Item {
            name: Some(name),
            inner,
            ..
        } => match inner {
            ItemEnum::Variant(_) => Some(Indexed {
                index: index as u32,
                value: Named {
                    name: name.clone(),
                    value: VariantFormat::Unit,
                },
            }),
            _ => None,
        },
        _ => None,
    }
}

fn make_struct_variant_format(
    variant: &ItemNode,
    fields: &Vec<(&Indexed<Named<Format>>,)>,
    all_variants: &Vec<ItemNode>,
) -> Option<Indexed<Named<VariantFormat>>> {
    let index = all_variants.iter().position(|f| f == variant)?;
    match &variant.0 {
        Item {
            name: Some(name),
            inner,
            ..
        } => match inner {
            ItemEnum::Variant(_) => {
                let mut fields = fields.clone();
                fields.sort();
                let fields = fields.iter().map(|(f,)| f.inner()).collect::<Vec<_>>();
                Some(Indexed {
                    index: index as u32,
                    value: Named {
                        name: name.clone(),
                        value: VariantFormat::Struct(fields),
                    },
                })
            }
            _ => None,
        },
        _ => None,
    }
}

fn make_tuple_variant_format(
    variant: &ItemNode,
    fields: &Vec<(&Indexed<Format>,)>,
    all_variants: &Vec<ItemNode>,
) -> Option<Indexed<Named<VariantFormat>>> {
    let index = all_variants.iter().position(|v| v == variant)?;
    match &variant.0 {
        Item {
            name: Some(name),
            inner,
            ..
        } => match inner {
            ItemEnum::Variant(_) => {
                let mut fields = fields.clone();
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
                        name: name.clone(),
                        value,
                    },
                })
            }
            _ => None,
        },
        _ => None,
    }
}

fn make_struct_unit() -> ContainerFormat {
    ContainerFormat::UnitStruct
}

fn make_struct_plain(fields: &Vec<(&Indexed<Named<Format>>,)>) -> ContainerFormat {
    let mut fields = fields.clone();
    fields.sort();
    let fields = fields.iter().map(|(f,)| f.inner()).collect::<Vec<_>>();
    match fields.len() {
        0 => ContainerFormat::UnitStruct,
        _ => ContainerFormat::Struct(fields),
    }
}

fn make_struct_tuple(fields: &Vec<(&Indexed<Format>,)>) -> ContainerFormat {
    let mut fields = fields.clone();
    fields.sort();
    let fields = fields.iter().map(|(f,)| f.inner()).collect::<Vec<_>>();
    match fields.len() {
        0 => ContainerFormat::UnitStruct,
        1 => ContainerFormat::NewTypeStruct(Box::new(fields[0].clone())),
        _ => ContainerFormat::TupleStruct(fields),
    }
}

fn make_enum(formats: &Vec<(&Indexed<Named<VariantFormat>>,)>) -> ContainerFormat {
    let mut map = BTreeMap::default();
    for (Indexed { index, value },) in formats.clone() {
        map.insert(*index, value.clone());
    }
    ContainerFormat::Enum(map)
}

impl From<&Type> for Format {
    fn from(type_: &Type) -> Self {
        match type_ {
            Type::ResolvedPath(path) => {
                if let Some(args) = &path.args {
                    match args.as_ref() {
                        GenericArgs::AngleBracketed {
                            args,
                            constraints: _,
                        } => match path.name.as_str() {
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
                            _ => Format::TypeName(path.name.clone()),
                        },
                        GenericArgs::Parenthesized {
                            inputs: _,
                            output: _,
                        } => todo!(),
                    }
                } else {
                    Format::TypeName(path.name.clone())
                }
            }
            Type::DynTrait(_dyn_trait) => todo!(),
            Type::Generic(_) => todo!(),
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
