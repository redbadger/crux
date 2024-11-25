use std::collections::BTreeMap;

use anyhow::Result;
use ascent::ascent;
use rustdoc_types::{
    Enum, GenericArg, GenericArgs, Impl, Item, ItemEnum, Path, Struct, StructKind, Type, Variant,
    VariantKind,
};
use serde::Serialize;

use crate::codegen::format::ContainerFormat;

use super::{
    data::{Data, Node},
    format::{Format, Named, VariantFormat},
};

ascent! {
    // facts

    relation edge(Node, Node, Edge);

    // rules

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

    relation subset(Node, Node);
    subset(root, child) <--
        root(root),
        edge(root, child, ?Edge::Variant|Edge::Field);
    subset(parent, child) <--
        subset(grandparent, parent),
        edge(parent, child, ?Edge::Variant|Edge::Field|Edge::Type);

    relation is_enum(Node);
    is_enum(enum_) <--
        subset(enum_, _),
        if let Some(Item { inner: ItemEnum::Enum(_), .. }) = &enum_.item;

    relation is_struct(Node);
    is_struct(struct_) <--
        subset(struct_, _),
        if let Some(Item { inner: ItemEnum::Struct(_), .. }) = &struct_.item;

    relation struct_plain(Node);
    struct_plain(struct_) <--
        is_struct(struct_),
        if let Some(Item { inner: ItemEnum::Struct(Struct{kind: StructKind::Plain { .. }, ..}), .. }) = &struct_.item;

    relation struct_tuple(Node);
    struct_tuple(struct_) <--
        is_struct(struct_),
        if let Some(Item { inner: ItemEnum::Struct(Struct{kind: StructKind::Tuple(_), ..}), .. }) = &struct_.item;

    relation struct_field(Node, Node);
    struct_field(struct_, field) <--
        is_struct(struct_),
        edge(struct_, field, Edge::Field);

    relation enum_variant(Node, Node);
    enum_variant(enum_, variant) <--
        is_enum(enum_),
        edge(enum_, variant, Edge::Variant);

    relation variant_plain(Node, Node);
    variant_plain(enum_, variant) <--
        enum_variant(enum_, variant),
        if let Some(Item { inner: ItemEnum::Variant(Variant{kind: VariantKind::Plain, ..}), .. }) = &variant.item;

    relation variant_struct(Node, Node);
    variant_struct(enum_, variant) <--
        enum_variant(enum_, variant),
        if let Some(Item { inner: ItemEnum::Variant(Variant{kind: VariantKind::Struct {..}, ..}), .. }) = &variant.item;

    relation variant_tuple(Node, Node);
    variant_tuple(enum_, variant) <--
        enum_variant(enum_, variant),
        if let Some(Item { inner: ItemEnum::Variant(Variant{kind: VariantKind::Tuple(_), ..}), .. }) = &variant.item;

    relation variant_field(Node, Node);
    variant_field(variant, field) <--
        enum_variant(enum_, variant),
        edge(variant, field, Edge::Field);

    relation format(Node, Format);
    format(struct_, format) <--
        struct_field(struct_, field),
        let format = make_format(&field);

    relation format_named(Node, Named<Format>);
    format_named(struct_, format) <--
        struct_field(struct_, field),
        let format = make_named_format(&field);

    relation format_plain_variant(Node, Named<VariantFormat>);
    format_plain_variant(enum_, format) <--
        variant_plain(enum_, variant),
        variant_field(variant, field),
        agg formats = collect(format) in format_named(field, format),
        let format = make_plain_variant_format(variant);

    relation format_struct_variant(Node, Named<VariantFormat>);
    format_struct_variant(enum_, format) <--
        variant_struct(enum_, variant),
        variant_field(variant, field),
        agg formats = collect(format) in format_named(field, format),
        let format = make_struct_variant_format(variant, &formats);

    relation format_tuple_variant(Node, Named<VariantFormat>);
    format_tuple_variant(enum_, format) <--
        variant_tuple(enum_, variant),
        variant_field(variant, field),
        agg formats = collect(format) in format(field, format),
        let format = make_tuple_variant_format(variant, &formats);

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
        agg formats = collect(format) in format_named(struct_, format),
        if let Some(Item { name: Some(n), .. }) = &struct_.item,
        let name = n.to_string(),
        let container = make_struct_plain(struct_, &formats);
    container(name, container) <--
        struct_tuple(struct_),
        agg formats = collect(format) in format(struct_, format),
        if let Some(Item { name: Some(n), .. }) = &struct_.item,
        let name = n.to_string(),
        let container = make_struct_tuple(&formats);
    container(name, container) <--
        is_enum(enum_),
        agg formats = collect(format) in format_variant(enum_, format),
        if let Some(Item { name: Some(n), .. }) = &enum_.item,
        let name = n.to_string(),
        let container = make_enum(&formats);
}

pub fn collect<'a, N: 'a, T: Iterator<Item = (&'a N,)>>(
    input: T,
) -> impl Iterator<Item = Vec<(&'a N,)>>
where
    N: Clone,
{
    std::iter::once(input.collect::<Vec<_>>())
}

fn make_format(field: &Node) -> Format {
    if let Some(item) = &field.item {
        if let ItemEnum::StructField(type_) = &item.inner {
            return type_.into();
        };
    }
    unreachable!()
}

fn make_named_format(field: &Node) -> Named<Format> {
    if let Some(Item {
        name: Some(name), ..
    }) = &field.item
    {
        return Named {
            name: name.clone(),
            value: make_format(field),
        };
    };
    unreachable!()
}

fn make_plain_variant_format(variant: &Node) -> Named<VariantFormat> {
    if let Some(Item {
        name: Some(name), ..
    }) = &variant.item
    {
        if let Some(item) = &variant.item {
            if let ItemEnum::Variant(_) = &item.inner {
                return Named {
                    name: name.clone(),
                    value: VariantFormat::Unit,
                };
            };
        }
    }
    unreachable!()
}

fn make_struct_variant_format(
    variant: &Node,
    fields: &Vec<(&Named<Format>,)>,
) -> Named<VariantFormat> {
    if let Some(Item {
        name: Some(name), ..
    }) = &variant.item
    {
        if let Some(item) = &variant.item {
            if let ItemEnum::Variant(_) = &item.inner {
                return Named {
                    name: name.clone(),
                    value: VariantFormat::Struct(
                        fields
                            .iter()
                            .map(|(field,)| (*field).clone())
                            .collect::<Vec<_>>(),
                    ),
                };
            };
        }
    }
    unreachable!()
}

fn make_tuple_variant_format(variant: &Node, fields: &Vec<(&Format,)>) -> Named<VariantFormat> {
    if let Some(Item {
        name: Some(name), ..
    }) = &variant.item
    {
        if let Some(item) = &variant.item {
            if let ItemEnum::Variant(_) = &item.inner {
                return Named {
                    name: name.clone(),
                    value: VariantFormat::Tuple(
                        fields
                            .iter()
                            .map(|(field,)| (*field).clone())
                            .collect::<Vec<_>>(),
                    ),
                };
            };
        }
    }
    unreachable!()
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

pub fn parse(data: &Data) -> Result<Vec<(String, ContainerFormat)>> {
    let mut prog = AscentProgram::default();

    // edges
    for (id, item) in &data.crate_.index {
        let Some(source) = data.node_by_id(id) else {
            continue;
        };
        if item.attrs.contains(&"#[serde(skip)]".to_string()) {
            continue;
        }

        match &item.inner {
            ItemEnum::Module(_module) => (),
            ItemEnum::ExternCrate { name: _, rename: _ } => (),
            ItemEnum::Use(_) => (),
            ItemEnum::Union(_union) => (),
            ItemEnum::Struct(s) => {
                match &s.kind {
                    StructKind::Unit => {
                        prog.edge
                            .push((source.clone(), source.clone(), Edge::Field))
                    }
                    StructKind::Tuple(fields) => {
                        for field in fields {
                            if let Some(id) = field {
                                let Some(dest) = data.node_by_id(id) else {
                                    continue;
                                };
                                prog.edge.push((source.clone(), dest.clone(), Edge::Field));
                            }
                        }
                    }
                    StructKind::Plain {
                        fields,
                        has_stripped_fields: _,
                    } => {
                        for id in fields {
                            let Some(dest) = data.node_by_id(id) else {
                                continue;
                            };
                            prog.edge.push((source.clone(), dest.clone(), Edge::Field));
                        }
                    }
                };
            }
            ItemEnum::StructField(type_) => match type_ {
                Type::ResolvedPath(path) => {
                    let Some(dest) = data.node_by_id(&path.id) else {
                        continue;
                    };
                    prog.edge.push((source.clone(), dest.clone(), Edge::Type));

                    if let Some(args) = &path.args {
                        process_args(source, args.as_ref(), &data, &mut prog.edge);
                    }
                }
                _ => (),
            },
            ItemEnum::Enum(Enum { variants, .. }) => {
                for id in variants {
                    let Some(dest) = data.node_by_id(id) else {
                        continue;
                    };
                    prog.edge
                        .push((source.clone(), dest.clone(), Edge::Variant));
                }
            }
            ItemEnum::Variant(v) => {
                match &v.kind {
                    VariantKind::Plain => {
                        prog.edge
                            .push((source.clone(), source.clone(), Edge::Field));
                    }
                    VariantKind::Tuple(fields) => {
                        for id in fields {
                            if let Some(id) = id {
                                let Some(dest) = data.node_by_id(id) else {
                                    continue;
                                };
                                prog.edge.push((source.clone(), dest.clone(), Edge::Field));
                            }
                        }
                    }
                    VariantKind::Struct {
                        fields,
                        has_stripped_fields: _,
                    } => {
                        for id in fields {
                            let Some(dest) = data.node_by_id(id) else {
                                continue;
                            };
                            prog.edge.push((source.clone(), dest.clone(), Edge::Field));
                        }
                    }
                };
            }
            ItemEnum::Function(_function) => (),
            ItemEnum::Trait(_) => (),
            ItemEnum::TraitAlias(_trait_alias) => (),
            ItemEnum::Impl(Impl {
                for_:
                    Type::ResolvedPath(Path {
                        id: for_type_id, ..
                    }),
                trait_:
                    Some(Path {
                        id: trait_id,
                        name: trait_name,
                        args: _,
                    }),
                items,
                ..
            }) => {
                let trait_edge = match trait_name.as_str() {
                    "App" => Edge::TraitApp,
                    "Effect" => Edge::TraitEffect,
                    _ => continue,
                };

                // record an edge for the trait the impl is of
                let Some(dest) = data.node_by_id(trait_id) else {
                    continue;
                };
                prog.edge.push((source.clone(), dest.clone(), trait_edge));

                // record an edge for the type the impl is for
                let Some(dest) = data.node_by_id(&for_type_id) else {
                    continue;
                };
                prog.edge.push((source.clone(), dest.clone(), Edge::Type));

                // record edges for the associated items in the impl
                for id in items {
                    let Some(dest) = data.node_by_id(id) else {
                        continue;
                    };

                    // skip everything except the Event and ViewModel associated types
                    if let Some(Item {
                        name: Some(name), ..
                    }) = &dest.item
                    {
                        if !["Event", "ViewModel", "Capabilities"].contains(&name.as_str()) {
                            continue;
                        }
                    }

                    prog.edge
                        .push((source.clone(), dest.clone(), Edge::AssociatedItem));
                }
            }
            ItemEnum::Impl(_) => (),
            ItemEnum::TypeAlias(_type_alias) => (),
            ItemEnum::Constant {
                type_: _,
                const_: _,
            } => (),
            ItemEnum::Static(_) => (),
            ItemEnum::ExternType => (),
            ItemEnum::Macro(_) => (),
            ItemEnum::ProcMacro(_proc_macro) => (),
            ItemEnum::Primitive(_primitive) => (),
            ItemEnum::AssocConst { type_: _, value: _ } => (),
            ItemEnum::AssocType {
                type_: Some(Type::ResolvedPath(target)),
                ..
            } => {
                // skip everything except the Event, ViewModel and Ffi associated types
                if let Item {
                    name: Some(name), ..
                } = &item
                {
                    if !["Event", "ViewModel", "Ffi"].contains(&name.as_str()) {
                        continue;
                    }
                }

                // record an edge for the associated type
                let Some(dest) = data.node_by_id(&target.id) else {
                    continue;
                };
                prog.edge
                    .push((source.clone(), dest.clone(), Edge::AssociatedType));
            }
            ItemEnum::AssocType { .. } => (),
        }
    }

    prog.run();

    Ok(prog.container)
}

fn process_args(
    source: &Node,
    args: &GenericArgs,
    data: &Data,
    edges: &mut Vec<(Node, Node, Edge)>,
) {
    if let GenericArgs::AngleBracketed { args, .. } = args {
        for arg in args {
            if let GenericArg::Type(t) = arg {
                if let Type::ResolvedPath(path) = t {
                    let Some(dest) = data.node_by_id(&path.id) else {
                        continue;
                    };
                    edges.push((source.clone(), dest.clone(), Edge::Type));

                    if let Some(args) = &path.args {
                        let generic_args = args.as_ref();
                        process_args(source, generic_args, data, edges);
                    }
                };
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
enum Edge {
    AssociatedItem,
    AssociatedType,
    Type,
    Field,
    Variant,
    TraitApp,
    TraitEffect,
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
                                    GenericArg::Type(ref t) => t.into(),
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
            Type::Tuple(_vec) => todo!(),
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
                name: _,
                args: _,
                self_type: _,
                trait_: _,
            } => todo!(),
        }
    }
}
