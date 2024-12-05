use std::hash::{Hash, Hasher};

use rustdoc_types::{
    Enum, GenericArg, GenericArgs, Id, Impl, Item, ItemEnum, ItemSummary, Path, Struct, StructKind,
    Type, Variant, VariantKind,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: Id,
    pub item: Option<Item>,
    pub summary: Option<ItemSummary>,
}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let crate_id = self.summary.as_ref().map(|s| &s.crate_id);
        (crate_id, self.id).hash(state);
    }
}

impl Node {
    pub fn name(&self) -> Option<&str> {
        self.item.as_ref().and_then(|item| {
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

    pub fn is_struct(&self) -> bool {
        matches!(
            &self.item,
            Some(Item {
                inner: ItemEnum::Struct(_),
                ..
            })
        )
    }

    pub fn is_struct_unit(&self) -> bool {
        matches!(
            &self.item,
            Some(Item {
                inner: ItemEnum::Struct(Struct {
                    kind: StructKind::Unit,
                    ..
                }),
                ..
            })
        )
    }

    pub fn is_struct_plain(&self) -> bool {
        matches!(
            &self.item,
            Some(Item {
                inner: ItemEnum::Struct(Struct {
                    kind: StructKind::Plain { .. },
                    ..
                }),
                ..
            })
        )
    }

    pub fn is_struct_tuple(&self) -> bool {
        matches!(
            &self.item,
            Some(Item {
                inner: ItemEnum::Struct(Struct {
                    kind: StructKind::Tuple(_),
                    ..
                }),
                ..
            })
        )
    }

    pub fn is_enum(&self) -> bool {
        matches!(
            &self.item,
            Some(Item {
                inner: ItemEnum::Enum(_),
                ..
            })
        )
    }

    pub fn is_impl_for(&self, for_: &Node, trait_name: &str) -> bool {
        match &self.item {
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

    fn should_skip(&self) -> bool {
        self.item
            .as_ref()
            .map(|i| {
                i.attrs.iter().any(|attr| {
                    lazy_regex::regex_is_match!(r#"\[serde\s*\(\s*skip\s*\)\s*\]"#, attr)
                })
            })
            .unwrap_or(false)
    }

    pub fn has_field(&self, field: &Node) -> bool {
        if field.should_skip() {
            return false;
        }

        match &self.item {
            Some(Item {
                inner: ItemEnum::Struct(Struct { kind, .. }),
                ..
            }) => {
                if field.name() == Some("__private_field") {
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

    pub fn has_variant(&self, variant: &Node) -> bool {
        if variant.should_skip() {
            return false;
        }

        match &self.item {
            Some(Item {
                inner: ItemEnum::Enum(Enum { variants, .. }),
                ..
            }) => variants.contains(&variant.id),
            _ => false,
        }
    }

    pub fn is_of_type(&self, type_node: &Node) -> bool {
        match &self.item {
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

    pub fn has_associated_item(&self, associated_item: &Node, with_name: &str) -> bool {
        match &self.item {
            Some(Item {
                inner: ItemEnum::Impl(Impl { items, .. }),
                ..
            }) => match &associated_item.item {
                Some(Item {
                    name: Some(name), ..
                }) if with_name == name => items.contains(&associated_item.id),
                _ => false,
            },
            _ => false,
        }
    }

    pub fn is_in_same_module_as(&self, other: &Node) -> bool {
        let (Some(this), Some(other)) = (&self.summary, &other.summary) else {
            return false;
        };

        if this.path.len() != other.path.len() {
            return false;
        };

        this.path[..(this.path.len() - 1)] == other.path[..(other.path.len() - 1)]
    }

    pub fn index_of(&self, child: &Node) -> Option<usize> {
        match &self.item {
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rustdoc_types::{Generics, Id, Item, ItemEnum, Struct, StructKind, Visibility};

    use super::*;

    fn test_node(name: Option<String>, attrs: Vec<String>) -> Node {
        Node {
            item: Some(Item {
                name,
                attrs,
                inner: ItemEnum::Struct(Struct {
                    kind: StructKind::Plain {
                        fields: vec![],
                        has_stripped_fields: false,
                    },
                    generics: Generics {
                        params: vec![],
                        where_predicates: vec![],
                    },
                    impls: vec![],
                }),
                id: Id(0),
                crate_id: 0,
                span: None,
                visibility: Visibility::Public,
                docs: None,
                links: Default::default(),
                deprecation: None,
            }),
            id: Id(0),
            summary: None,
        }
    }

    #[test]
    fn test_get_name() {
        let name = Some("Foo".to_string());
        let attrs = vec![];
        let node = test_node(name, attrs);
        assert_eq!(node.name(), Some("Foo"));
    }

    #[test]
    fn test_get_name_with_rename() {
        let name = Some("Foo".to_string());
        let attrs = vec![r#"#[serde(rename = "Bar")]"#.to_string()];
        let node = test_node(name, attrs);
        assert_eq!(node.name(), Some("Bar"));
    }

    #[test]
    fn test_get_name_with_rename_no_whitespace() {
        let name = Some("Foo".to_string());
        let attrs = vec![r#"#[serde(rename="Bar")]"#.to_string()];
        let node = test_node(name, attrs);
        assert_eq!(node.name(), Some("Bar"));
    }

    #[test]
    fn test_get_name_with_no_name() {
        let name = None;
        let attrs = vec![];
        let node = test_node(name, attrs);
        assert_eq!(node.name(), None);
    }
}
