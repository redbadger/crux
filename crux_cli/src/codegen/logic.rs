use std::collections::BTreeMap;

use ascent::ascent_run;
use rustdoc_types::{
    GenericArg, GenericArgs, Item, ItemEnum, Struct, StructKind, Type, Variant, VariantKind,
};

use crate::codegen::format::ContainerFormat;

use super::{
    data::{Edge, Node},
    format::{Format, Named, VariantFormat},
};

pub fn run(data: Vec<(Node, Node, Edge)>) -> Vec<(String, ContainerFormat)> {
    let prog = ascent_run! {
        // ------- facts ------------------

        relation edge(Node, Node, Edge) = data;

        // ------- rules over input -------
        // filters out the items that are not of interest

        relation app(Node);
        // app structs have an implementation of the App trait
        app(app) <--
            edge(app_impl, app_trait, Edge::TraitApp),
            edge(app_impl, app, Edge::Type);

        relation effect(Node);
        // effect enums have an implementation of the Effect trait
        effect(effect) <--
            edge(effect_impl, effect_trait, Edge::TraitEffect),
            edge(effect_impl, effect, Edge::Type);

        relation is_effect_of_app(Node, Node);
        // an effect belongs to an app if they are in the same module
        is_effect_of_app(app, effect) <--
            app(app),
            effect(effect),
            if are_in_same_module(app, effect);

        relation root(Node);
        // Event and ViewModel types are associated
        // with the root apps (that have no parent)
        root(assoc_type) <--
            edge(app_impl, app_trait, Edge::TraitApp),
            edge(app_impl, app, Edge::Type),
            !parent(_, app),
            edge(app_impl, assoc_item, Edge::AssociatedItem),
            edge(assoc_item, assoc_type, Edge::AssociatedType);
        // Effects belong to the root apps (that have no parent)
        root(effect_enum) <--
            is_effect_of_app(app, effect_enum),
            !parent(_, app);

        relation parent(Node, Node);
        // app hierarchy
        parent(parent, child) <--
            app(parent),
            app(child),
            edge(parent, field, Edge::Field),
            edge(field, child, Edge::Type);

        // set of all the edges we are interested in
        relation subset(Node, Node);
        subset(root, child) <--
            root(root),
            edge(root, child, ?Edge::Variant|Edge::Field);
        subset(parent, child) <--
            subset(grandparent, parent),
            edge(parent, child, ?Edge::Variant|Edge::Field|Edge::Type);

        relation struct_in_subset(Node);
        struct_in_subset(struct_) <--
            subset(struct_, _),
            if let Some(Item { inner: ItemEnum::Struct(_), .. }) = &struct_.item;

        relation enum_in_subset(Node);
        enum_in_subset(enum_) <--
            subset(enum_, _),
            if let Some(Item { inner: ItemEnum::Enum(_), .. }) = &enum_.item;

        relation struct_plain(Node);
        struct_plain(struct_) <--
            struct_in_subset(struct_),
            if let Some(Item { inner: ItemEnum::Struct(Struct{kind: StructKind::Plain { .. }, ..}), .. }) = &struct_.item;

        relation struct_tuple(Node);
        struct_tuple(struct_) <--
            struct_in_subset(struct_),
            if let Some(Item { inner: ItemEnum::Struct(Struct{kind: StructKind::Tuple(_), ..}), .. }) = &struct_.item;

        relation variant(Node, Node);
        variant(enum_, variant) <--
            enum_in_subset(enum_),
            edge(enum_, variant, Edge::Variant);

        relation field(Node, Node);
        field(struct_or_variant, field) <--
            (struct_in_subset(struct_or_variant) || variant(enum_, struct_or_variant)),
            edge(struct_or_variant, field, Edge::Field);

        relation variant_plain(Node, Node);
        variant_plain(enum_, variant) <--
            variant(enum_, variant),
            if is_plain_variant(&variant);

        relation variant_tuple(Node, Node);
        variant_tuple(enum_, variant) <--
            variant(enum_, variant),
            if is_tuple_variant(&variant);

        relation variant_struct(Node, Node);
        variant_struct(enum_, variant) <--
            variant(enum_, variant),
            if is_struct_variant(&variant);

        // ------- rules over output -------
        // these rules are used to generate the output

        relation format(Node, Format);
        format(struct_or_variant, format) <--
            field(struct_or_variant, field),
            if let Some(format) = make_format(&field);

        relation format_named(Node, Named<Format>);
        format_named(struct_or_variant, format) <--
            field(struct_or_variant, field),
            if let Some(format) = make_named_format(&field);

        relation format_plain_variant(Node, Named<VariantFormat>);
        format_plain_variant(enum_, format) <--
            variant_plain(enum_, variant),
            if let Some(format) = make_plain_variant_format(variant);

        relation format_tuple_variant(Node, Named<VariantFormat>);
        format_tuple_variant(enum_, format) <--
            variant_tuple(enum_, variant),
            field(variant, field),
            agg formats = collect(format) in format(field, format),
            if let Some(format) = make_tuple_variant_format(variant, &formats);

        relation format_struct_variant(Node, Named<VariantFormat>);
        format_struct_variant(enum_, format) <--
            variant_struct(enum_, variant),
            field(variant, field),
            agg formats = collect(format) in format_named(field, format),
            if let Some(format) = make_struct_variant_format(variant, &formats);

        relation format_variant(Node, Named<VariantFormat>);
        format_variant(enum_, format) <--
            (
                format_plain_variant(enum_, format) ||
                format_struct_variant(enum_, format) ||
                format_tuple_variant(enum_, format)
            );

        relation container(String, ContainerFormat);
        container(name, container) <--
            struct_plain(struct_),
            agg fields = collect(format) in format_named(struct_, format),
            if let Some(Item { name: Some(n), .. }) = &struct_.item,
            let name = n.to_string(),
            let container = make_struct_plain(struct_, &fields);
        container(name, container) <--
            struct_tuple(struct_),
            agg fields = collect(format) in format(struct_, format),
            if let Some(Item { name: Some(n), .. }) = &struct_.item,
            let name = n.to_string(),
            let container = make_struct_tuple(&fields);
        container(name, container) <--
            enum_in_subset(enum_),
            agg variants = collect(format) in format_variant(enum_, format),
            if let Some(Item { name: Some(n), .. }) = &enum_.item,
            let name = n.to_string(),
            let container = make_enum(&variants);
    };

    // write field and variant edges to disk for debugging
    for (name, contents) in &[
        ("edge.json", serde_json::to_string(&prog.edge).unwrap()),
        ("app.json", serde_json::to_string(&prog.app).unwrap()),
        ("effect.json", serde_json::to_string(&prog.effect).unwrap()),
        (
            "is_effect_of_app.json",
            serde_json::to_string(&prog.is_effect_of_app).unwrap(),
        ),
        ("root.json", serde_json::to_string(&prog.root).unwrap()),
        ("parent.json", serde_json::to_string(&prog.parent).unwrap()),
        ("subset.json", serde_json::to_string(&prog.subset).unwrap()),
        (
            "enum_in_subset.json",
            serde_json::to_string(&prog.enum_in_subset).unwrap(),
        ),
        (
            "struct_in_subset.json",
            serde_json::to_string(&prog.struct_in_subset).unwrap(),
        ),
        ("field.json", serde_json::to_string(&prog.field).unwrap()),
        (
            "struct_plain.json",
            serde_json::to_string(&prog.struct_plain).unwrap(),
        ),
        (
            "struct_tuple.json",
            serde_json::to_string(&prog.struct_tuple).unwrap(),
        ),
        (
            "variant.json",
            serde_json::to_string(&prog.variant).unwrap(),
        ),
        (
            "variant_plain.json",
            serde_json::to_string(&prog.variant_plain).unwrap(),
        ),
        (
            "variant_struct.json",
            serde_json::to_string(&prog.variant_struct).unwrap(),
        ),
        (
            "variant_tuple.json",
            serde_json::to_string(&prog.variant_tuple).unwrap(),
        ),
        ("format.txt", format!("{:#?}", &prog.format)),
        ("format_named.txt", format!("{:#?}", &prog.format_named)),
        (
            "format_plain_variant.txt",
            format!("{:#?}", &prog.format_plain_variant),
        ),
        (
            "format_tuple_variant.txt",
            format!("{:#?}", &prog.format_tuple_variant),
        ),
        (
            "format_struct_variant.txt",
            format!("{:#?}", &prog.format_struct_variant),
        ),
        ("container.txt", format!("{:#?}", &prog.container)),
    ] {
        std::fs::write(format!("/tmp/stu/{name}"), contents).unwrap();
    }

    prog.container
}

pub fn collect<'a, N: 'a, T: Iterator<Item = (&'a N,)>>(
    input: T,
) -> impl Iterator<Item = Vec<(&'a N,)>>
where
    N: Clone,
{
    std::iter::once(input.collect::<Vec<_>>())
}

fn make_format(field: &Node) -> Option<Format> {
    match &field.item {
        Some(item) => match &item.inner {
            ItemEnum::StructField(type_) => Some(type_.into()),
            _ => None,
        },
        _ => None,
    }
}

fn make_named_format(field: &Node) -> Option<Named<Format>> {
    match &field.item {
        Some(Item {
            name: Some(name), ..
        }) => match make_format(field) {
            Some(value) => Some(Named {
                name: name.clone(),
                value,
            }),
            _ => None,
        },
        _ => None,
    }
}

fn is_plain_variant(variant: &Node) -> bool {
    matches!(
        &variant.item,
        Some(Item {
            inner: ItemEnum::Variant(Variant {
                kind: VariantKind::Plain,
                ..
            }),
            ..
        })
    )
}

fn is_struct_variant(variant: &Node) -> bool {
    matches!(
        &variant.item,
        Some(Item {
            inner: ItemEnum::Variant(Variant {
                kind: VariantKind::Struct { .. },
                ..
            }),
            ..
        })
    )
}

fn is_tuple_variant(variant: &Node) -> bool {
    matches!(
        &variant.item,
        Some(Item {
            inner: ItemEnum::Variant(Variant {
                kind: VariantKind::Tuple(_),
                ..
            }),
            ..
        })
    )
}

fn make_plain_variant_format(variant: &Node) -> Option<Named<VariantFormat>> {
    match &variant.item {
        Some(Item {
            name: Some(name),
            inner,
            ..
        }) => match inner {
            ItemEnum::Variant(_) => Some(Named {
                name: name.clone(),
                value: VariantFormat::Unit,
            }),
            _ => None,
        },
        _ => None,
    }
}

fn make_struct_variant_format(
    variant: &Node,
    fields: &Vec<(&Named<Format>,)>,
) -> Option<Named<VariantFormat>> {
    match &variant.item {
        Some(Item {
            name: Some(name),
            inner,
            ..
        }) => match inner {
            ItemEnum::Variant(_) => Some(Named {
                name: name.clone(),
                value: VariantFormat::Struct(
                    fields
                        .iter()
                        .map(|(field,)| (*field).clone())
                        .collect::<Vec<_>>(),
                ),
            }),
            _ => None,
        },
        _ => None,
    }
}

fn make_tuple_variant_format(
    variant: &Node,
    fields: &Vec<(&Format,)>,
) -> Option<Named<VariantFormat>> {
    match &variant.item {
        Some(Item {
            name: Some(name),
            inner,
            ..
        }) => match inner {
            ItemEnum::Variant(_) => Some(Named {
                name: name.clone(),
                value: VariantFormat::Tuple(
                    fields
                        .iter()
                        .map(|(field,)| (*field).clone())
                        .collect::<Vec<_>>(),
                ),
            }),
            _ => None,
        },
        _ => None,
    }
}

fn make_struct_plain(_node: &Node, fields: &Vec<(&Named<Format>,)>) -> ContainerFormat {
    ContainerFormat::Struct(
        fields
            .iter()
            .map(|(field,)| (*field).clone())
            .collect::<Vec<_>>(),
    )
}

fn make_struct_tuple(fields: &Vec<(&Format,)>) -> ContainerFormat {
    ContainerFormat::TupleStruct(
        fields
            .iter()
            .map(|(field,)| (*field).clone())
            .collect::<Vec<_>>(),
    )
}

fn make_enum(variants: &Vec<(&Named<VariantFormat>,)>) -> ContainerFormat {
    ContainerFormat::Enum(
        variants
            .iter()
            .enumerate()
            .map(|(i, (variant,))| (i as u32, (*variant).clone()))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn are_in_same_module(app: &Node, effect: &Node) -> bool {
    let (Some(app), Some(effect)) = (&app.summary, &effect.summary) else {
        return false;
    };
    if app.path.len() != effect.path.len() {
        return false;
    };
    app.path[..(app.path.len() - 1)] == effect.path[..(effect.path.len() - 1)]
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
                        } => {
                            if path.name == "Option" {
                                let format = match args[0] {
                                    GenericArg::Type(ref type_) => type_.into(),
                                    _ => todo!(),
                                };
                                Format::Option(Box::new(format))
                            } else {
                                Format::TypeName(path.name.clone())
                            }
                        }
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

#[cfg(test)]
#[path = "logic_tests.rs"]
mod logic_tests;
