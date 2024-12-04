use std::{
    cmp::Ordering,
    collections::BTreeMap,
    hash::{Hash, Hasher},
};

use ascent::ascent;
use rustdoc_types::{
    Enum, GenericArg, GenericArgs, Id, Impl, Item, ItemEnum, ItemSummary, Path, Struct, StructKind,
    Type, Variant, VariantKind,
};
use serde::{Deserialize, Serialize};

use super::{
    format::{ContainerFormat, Format, Named, VariantFormat},
    Registry,
};

ascent! {
    struct Filter;

    // ------- facts ------------------
    relation node(Node);

    // ------- rules ------------------

    relation is_struct(Node);
    is_struct(struct_) <--
        node(struct_),
        if let Some(Item { inner: ItemEnum::Struct(_), .. }) = &struct_.item;

    relation is_enum(Node);
    is_enum(enum_) <--
        node(enum_),
        if let Some(Item { inner: ItemEnum::Enum(_), .. }) = &enum_.item;

    relation field(Node, Node);
    field(parent, field) <--
        node(parent),
        node(field),
        if is_field_of(parent, field);

    relation variant(Node, Node);
    variant(parent, variant) <--
        is_enum(parent),
        node(variant),
        if is_variant_of(parent, variant);

    relation type_of(Node, Node);
    type_of(parent, type_) <--
        node(parent),
        node(type_),
        if is_type_of(parent, type_);

    // app structs have an implementation of the App trait
    relation app(Node, Node);
    app(impl_, app) <--
        node(impl_),
        is_struct(app),
        if is_impl_of(impl_, app, "App");

    // app hierarchy
    relation parent(Node, Node);
    parent(parent, child) <--
        app(_, parent),
        app(_, child),
        field(parent, field),
        type_of(field, child);

    relation root_app(Node, Node);
    root_app(impl_, app) <--
        app(impl_, app),
        !parent(_, app);

    // a view model is an associated type of an app
    relation view_model(Node, Node);
    view_model(app, view_model) <--
        root_app(impl_, app),
        type_of(item, view_model),
        if is_associated_item(impl_, item, "ViewModel");

    // an event is an associated type of an app
    relation event(Node, Node);
    event(app, event) <--
        root_app(impl_, app),
        type_of(item, event),
        if is_associated_item(impl_, item, "Event");

    // effect enums have an implementation of the Effect trait
    // and an associated Ffi type, which is the FFI representation of the effect
    relation effect(Node, Node);
    effect(app, effect_ffi) <--
        root_app(app_impl, app),
        is_enum(effect),
        node(effect_impl),
        if is_impl_of(effect_impl, effect, "Effect"),
        if are_in_same_module(app, effect),
        type_of(effect_ffi_item, effect_ffi),
        if is_associated_item(effect_impl, effect_ffi_item, "Ffi");

    relation root(Node);
    root(x) <-- view_model(app, x);
    root(x) <-- event(app, x);
    root(x) <-- effect(app, x);

    // set of all the edges we are interested in
    relation edge(Node, Node);

    // root fields
    edge(root, field) <--
        root(root),
        field(root, field);
    // root variants
    edge(root, variant) <--
        root(root),
        variant(root, variant);

    edge(type_, field) <--
        edge(_, type_),
        field(type_, field);
    edge(type_, variant) <--
        edge(_, type_),
        variant(type_, variant);

    edge(field, type_) <--
        edge(_, field),
        type_of(field, type_);
}

ascent! {
    struct Formatter;

    // ------- facts ------------------
    relation edge(Node, Node);

    // ------- rules ------------------

    relation struct_plain(Node);
    struct_plain(struct_) <--
        edge(struct_, _),
        if is_plain_struct(struct_);

    relation struct_tuple(Node);
    struct_tuple(struct_) <--
        edge(struct_, _),
        if is_tuple_struct(struct_);

    relation field(Node, Node);
    field(struct_or_variant, field) <--
        edge(struct_or_variant, field),
        if is_field_of(struct_or_variant, field);

    relation variant(Node, Node);
    variant(enum_, variant) <--
        edge(enum_, variant),
        if is_variant_of(enum_, variant);

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

    relation format(Node, Indexed<Format>);
    format(struct_or_variant, format) <--
        field(struct_or_variant, field),
        if let Some(format) = make_format(struct_or_variant, &field);

    relation format_named(Node, Indexed<Named<Format>>);
    format_named(struct_or_variant, format) <--
        field(struct_or_variant, field),
        if let Some(format) = make_named_format(struct_or_variant, &field);

    relation format_plain_variant(Node, Indexed<Named<VariantFormat>>);
    format_plain_variant(enum_, format) <--
        variant_plain(enum_, variant),
        if let Some(format) = make_plain_variant_format(enum_, variant);

    relation format_tuple_variant(Node, Indexed<Named<VariantFormat>>);
    format_tuple_variant(enum_, format) <--
        variant_tuple(enum_, variant),
        agg formats = collect(format) in format(variant, format),
        if let Some(format) = make_tuple_variant_format(enum_, variant, &formats);

    relation format_struct_variant(Node, Indexed<Named<VariantFormat>>);
    format_struct_variant(enum_, format) <--
        variant_struct(enum_, variant),
        agg formats = collect(format) in format_named(variant, format),
        if let Some(format) = make_struct_variant_format(enum_, variant, &formats);

    relation format_variant(Node, Indexed<Named<VariantFormat>>);
    format_variant(enum_, format) <-- format_plain_variant(enum_, format);
    format_variant(enum_, format) <-- format_tuple_variant(enum_, format);
    format_variant(enum_, format) <-- format_struct_variant(enum_, format);

    relation container(String, ContainerFormat);
    container(name, container) <--
        struct_plain(struct_),
        agg fields = collect(format) in format_named(struct_, format),
        if let Some(name) = name_of(struct_),
        let container = make_struct_plain(&fields);
    container(name, container) <--
        struct_tuple(struct_),
        agg fields = collect(format) in format(struct_, format),
        if let Some(name) = name_of(struct_),
        let container = make_struct_tuple(&fields);
    container(name, container) <--
        variant(enum_, _),
        agg variants = collect(format) in format_variant(enum_, format),
        if let Some(name) = name_of(enum_),
        let container = make_enum(&variants);
}

pub fn run(nodes: Vec<(Node,)>) -> Registry {
    let mut filter = Filter::default();
    filter.node = nodes;
    filter.run();

    let mut formatter = Formatter::default();
    formatter.edge = filter.edge;
    formatter.run();

    // std::fs::write("root.txt", format!("{:#?}", prog.root)).unwrap();
    formatter.container.into_iter().collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: Id,
    pub item: Option<Item>,
    pub summary: Option<ItemSummary>,
}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// An indexed value.
/// Used for preserving member position in parent type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Indexed<T> {
    pub index: u32,
    pub value: T,
}

impl<T: Clone> Indexed<T> {
    fn inner(&self) -> T {
        self.value.clone()
    }
}

impl<T: Eq> Ord for Indexed<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

impl<T: Eq> PartialOrd for Indexed<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn is_impl_of(impl_: &Node, for_: &Node, trait_name: &str) -> bool {
    match &impl_.item {
        Some(Item {
            inner:
                ItemEnum::Impl(Impl {
                    trait_: Some(Path { name, .. }),
                    for_: Type::ResolvedPath(Path { id, .. }),
                    ..
                }),
            ..
        }) if name == trait_name && id == &for_.id => true,
        _ => false,
    }
}

fn is_field_of(parent: &Node, field: &Node) -> bool {
    match &parent.item {
        Some(Item {
            inner: ItemEnum::Struct(Struct { kind, .. }),
            ..
        }) => {
            if name_of(field) == Some("__private_field") {
                return false;
            }
            match kind {
                StructKind::Unit => false,
                StructKind::Tuple(fields) => fields.contains(&Some(field.id)),
                StructKind::Plain {
                    fields,
                    has_stripped_fields: _,
                } => fields.contains(&field.id),
            }
        }
        Some(Item {
            inner: ItemEnum::Variant(Variant { kind, .. }),
            ..
        }) => match kind {
            VariantKind::Plain => false,
            VariantKind::Tuple(fields) => fields.contains(&Some(field.id)),
            VariantKind::Struct { fields, .. } => fields.contains(&field.id),
        },
        _ => false,
    }
}

fn is_variant_of(enum_: &Node, variant: &Node) -> bool {
    match &enum_.item {
        Some(Item {
            inner: ItemEnum::Enum(Enum { variants, .. }),
            ..
        }) => variants.contains(&variant.id),
        _ => false,
    }
}

fn is_type_of(field_node: &Node, type_node: &Node) -> bool {
    match &field_node.item {
        Some(Item {
            inner: ItemEnum::StructField(t),
            ..
        }) => check_type(type_node, t),
        Some(Item {
            inner:
                ItemEnum::AssocType {
                    type_: Some(Type::ResolvedPath(target)),
                    ..
                },
            ..
        }) => target.id == type_node.id,
        _ => false,
    }
}

fn check_type(type_node: &Node, type_: &Type) -> bool {
    match type_ {
        Type::ResolvedPath(Path { id, args, .. }) => {
            id == &type_node.id || check_args(type_node, args)
        }
        Type::Primitive(_) => false,
        Type::Tuple(vec) => vec.iter().any(|t| check_type(type_node, t)),
        Type::Slice(t) => check_type(type_node, t),
        Type::Array { type_: t, .. } => check_type(type_node, t),
        _ => false,
    }
}

fn check_args(type_node: &Node, args: &Option<Box<GenericArgs>>) -> bool {
    match args.as_deref() {
        Some(GenericArgs::AngleBracketed { args, .. }) => args
            .iter()
            .any(|arg| matches!(arg, GenericArg::Type(t) if check_type(type_node, t))),
        _ => false,
    }
}

fn is_associated_item(impl_: &Node, associated_item: &Node, name: &str) -> bool {
    match &impl_.item {
        Some(Item {
            inner: ItemEnum::Impl(Impl { items, .. }),
            ..
        }) => match &associated_item.item {
            Some(Item { name: Some(n), .. }) if name == n => items.contains(&associated_item.id),
            _ => false,
        },
        _ => false,
    }
}

pub fn collect<'a, N: 'a, T: Iterator<Item = (&'a N,)>>(
    input: T,
) -> impl Iterator<Item = Vec<(&'a N,)>>
where
    N: Clone,
{
    std::iter::once(input.collect::<Vec<_>>())
}

fn name_of(node: &Node) -> Option<&str> {
    node.item.as_ref().and_then(|item| {
        let mut new_name = "";
        for attr in &item.attrs {
            if let Some((_, n)) =
                lazy_regex::regex_captures!(r#"\[serde\(rename\s*=\s*"(\w+)"\)\]"#, attr)
            {
                new_name = n;
            }
        }
        if new_name.is_empty() {
            item.name.as_deref()
        } else {
            Some(new_name)
        }
    })
}

fn make_format(node: &Node, field: &Node) -> Option<Indexed<Format>> {
    match &field.item {
        Some(item) => match &item.inner {
            ItemEnum::StructField(type_) => Some(Indexed {
                index: index(node, field)? as u32,
                value: type_.into(),
            }),
            _ => None,
        },
        _ => None,
    }
}

fn make_named_format(node: &Node, field: &Node) -> Option<Indexed<Named<Format>>> {
    match name_of(field) {
        Some(name) => match make_format(node, field) {
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

fn is_plain_struct(struct_: &Node) -> bool {
    matches!(
        &struct_.item,
        Some(Item {
            inner: ItemEnum::Struct(Struct {
                kind: StructKind::Plain { .. },
                ..
            }),
            ..
        })
    )
}

fn is_tuple_struct(struct_: &Node) -> bool {
    matches!(
        &struct_.item,
        Some(Item {
            inner: ItemEnum::Struct(Struct {
                kind: StructKind::Tuple { .. },
                ..
            }),
            ..
        })
    )
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

fn make_plain_variant_format(
    enum_: &Node,
    variant: &Node,
) -> Option<Indexed<Named<VariantFormat>>> {
    match &variant.item {
        Some(Item {
            name: Some(name),
            inner,
            ..
        }) => match inner {
            ItemEnum::Variant(_) => Some(Indexed {
                index: index(enum_, variant)? as u32,
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
    enum_: &Node,
    variant: &Node,
    fields: &Vec<(&Indexed<Named<Format>>,)>,
) -> Option<Indexed<Named<VariantFormat>>> {
    match &variant.item {
        Some(Item {
            name: Some(name),
            inner,
            ..
        }) => match inner {
            ItemEnum::Variant(_) => {
                let mut fields = fields.clone();
                fields.sort();
                let fields = fields.iter().map(|(f,)| f.inner()).collect::<Vec<_>>();
                Some(Indexed {
                    index: index(enum_, variant)? as u32,
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
    enum_: &Node,
    variant: &Node,
    fields: &Vec<(&Indexed<Format>,)>,
) -> Option<Indexed<Named<VariantFormat>>> {
    match &variant.item {
        Some(Item {
            name: Some(name),
            inner,
            ..
        }) => match inner {
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
                    index: index(enum_, variant)? as u32,
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

fn index(node: &Node, child: &Node) -> Option<usize> {
    match &node.item {
        Some(Item {
            inner: ItemEnum::Enum(Enum { variants, .. }),
            ..
        }) => variants.iter().position(|v| v == &child.id),
        Some(Item {
            inner: ItemEnum::Struct(Struct { kind, .. }),
            ..
        }) => match kind {
            StructKind::Plain { fields, .. } => fields.iter().position(|f| f == &child.id),
            StructKind::Tuple(fields) => fields.iter().position(|f| f == &Some(child.id)),
            StructKind::Unit => None,
        },
        Some(Item {
            inner: ItemEnum::Variant(Variant { kind, .. }),
            ..
        }) => match kind {
            VariantKind::Plain => None,
            VariantKind::Tuple(fields) => fields.iter().position(|f| f == &Some(child.id)),
            VariantKind::Struct { fields, .. } => fields.iter().position(|f| f == &child.id),
        },
        _ => None,
    }
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

#[cfg(test)]
#[path = "logic_tests.rs"]
mod logic_tests;
