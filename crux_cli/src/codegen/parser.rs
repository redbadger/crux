use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use rustdoc_types::{Crate, Id, Impl, ItemEnum, Path, Type};

use super::{
    public_api::PublicApi,
    rust_types::{RustEnum, RustStruct, RustTypeAlias},
};
use crate::codegen::{
    item_processor::{sorting_prefix, ItemProcessor},
    nameable_item::NameableItem,
    path_component::PathComponent,
    public_item::PublicItem,
    render::RenderingContext,
    rust_types::{
        self, RustEnumShared, RustEnumVariant, RustEnumVariantShared, RustField, RustType,
    },
};

/// The results of parsing Rust source input.
#[derive(Default, Debug)]
pub struct ParsedData {
    /// Structs defined in the source
    pub structs: Vec<RustStruct>,
    /// Enums defined in the source
    pub enums: Vec<RustEnum>,
    /// Type aliases defined in the source
    pub aliases: Vec<RustTypeAlias>,
}

impl ParsedData {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug, Clone)]
enum Value {
    Namespace,
    Struct(RustStruct),
    Enum(RustEnum),
    TypeAlias(RustTypeAlias),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Component {
    type_: ComponentType,
    name: String,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum ComponentType {
    Enum,
    Struct,
    StructField,
    TupleVariantField,
    Variant,
    TypeAlias,
    Namespace,
}

impl Component {
    fn child_depth(&self) -> usize {
        match self.type_ {
            ComponentType::Enum => 1,
            ComponentType::Struct => 1,
            ComponentType::StructField => 2,
            ComponentType::TupleVariantField => 3,
            ComponentType::Variant => 2,
            ComponentType::TypeAlias => 1,
            ComponentType::Namespace => 0,
        }
    }
}

impl TryFrom<&str> for Component {
    type Error = anyhow::Error;

    fn try_from(component: &str) -> Result<Self, Self::Error> {
        let Some((type_, name)) = component.split_once('-') else {
            bail!("malformed component: {}", component);
        };
        if type_.len() != 3 {
            bail!("Invalid type: {}", type_);
        }
        let type_ = match type_.parse::<u8>()? {
            7 => ComponentType::Enum,
            9 => ComponentType::Struct,
            10 => {
                if name.parse::<usize>().is_ok() {
                    ComponentType::TupleVariantField
                } else {
                    ComponentType::StructField
                }
            }
            11 => ComponentType::Variant,
            19 => ComponentType::TypeAlias,
            21 => ComponentType::Namespace,
            _ => bail!("Unknown type: {}", type_),
        };
        let name = name
            .split_whitespace()
            .last()
            .ok_or_else(|| anyhow!("malformed component: {}", name))?
            .to_string();
        Ok(Self { type_, name })
    }
}

pub fn parse(crate_: &Crate) -> Result<ParsedData> {
    let mut item_processor = ItemProcessor::new(crate_);
    add_items(crate_, &mut item_processor, "Effect", &["Ffi"]);
    add_items(crate_, &mut item_processor, "App", &["Event", "ViewModel"]);
    item_processor.run();

    let context = RenderingContext {
        crate_,
        id_to_items: item_processor.id_to_items(),
    };

    let items: Vec<_> = item_processor
        .output
        .iter()
        .filter_map(|item| {
            matches!(
                &item.item().inner,
                ItemEnum::Union(_)
                    | ItemEnum::Struct(_)
                    | ItemEnum::StructField(_)
                    | ItemEnum::Enum(_)
                    | ItemEnum::Variant(_)
                    | ItemEnum::Primitive(_)
                    | ItemEnum::TypeAlias(_)
                    | ItemEnum::ForeignType
            )
            .then_some(PublicItem::from_intermediate_public_item(&context, item))
        })
        .collect();

    let mut public_api = PublicApi {
        items,
        missing_item_ids: item_processor.crate_.missing_item_ids(),
    };

    public_api.items.sort_by(PublicItem::grouping_cmp);

    println!();

    let mut data = HashMap::new();

    for item in public_api.items.iter().rev() {
        println!("{}", item);
        println!("{:?}", item.sortable_path);

        let (subject, object) = parse_sortable_path(item.sortable_path.as_slice())?;

        println!("{subject:?}\n{object:?}");

        let entry = data.entry(subject.clone()).or_insert_with(|| {
            let component = subject.last().unwrap();
            let id = rust_types::Id {
                original: component.name.clone(),
                renamed: component.name.clone(), // TODO: serde rename
            };
            match component.type_ {
                ComponentType::Enum => Value::Enum(RustEnum::Algebraic {
                    tag_key: component.name.clone(),
                    content_key: String::new(),
                    shared: RustEnumShared {
                        id,
                        generic_types: Vec::new(),
                        comments: Vec::new(),
                        variants: Vec::new(),
                        is_recursive: false,
                    },
                }),
                ComponentType::Struct => Value::Struct(RustStruct {
                    id,
                    generic_types: Vec::new(),
                    fields: Vec::new(),
                    comments: Vec::new(),
                }),
                _ => unimplemented!(),
            }
        });

        let mut variant_type: Option<RustEnumVariant> = None;

        for component in object.iter().rev() {
            let id = rust_types::Id {
                original: component.name.clone(),
                renamed: component.name.clone(), // TODO: serde rename
            };

            match component.type_ {
                ComponentType::StructField => {
                    println!("adding StructField");
                    let Value::Struct(ref mut value) = entry else {
                        bail!("entry is not a struct {:#?}", entry);
                    };

                    value.fields.push(RustField {
                        id,
                        ty: RustType::Simple {
                            id: component.name.clone(),
                        },
                        comments: Vec::new(),
                        has_default: false,
                    });
                }
                ComponentType::Variant => {
                    println!("adding Variant");
                    let Value::Enum(ref mut value) = entry else {
                        bail!("entry is not an enum {:#?}", entry);
                    };

                    match value {
                        RustEnum::Algebraic {
                            shared:
                                RustEnumShared {
                                    id: _,
                                    generic_types: _,
                                    comments: _,
                                    ref mut variants,
                                    is_recursive: _,
                                },
                            ..
                        } => {
                            if !variants.iter().any(|v| v.shared().id == id) {
                                if let Some(variant_type) = variant_type.take() {
                                    variants.push(variant_type);
                                } else {
                                    variants.push(RustEnumVariant::AnonymousStruct {
                                        fields: Vec::new(),
                                        shared: RustEnumVariantShared {
                                            id,
                                            comments: Vec::new(),
                                        },
                                    });
                                }
                            }
                        }
                        _ => bail!("unexpected enum type"),
                    }
                }
                ComponentType::TupleVariantField => {
                    println!("adding TupleVariantField");
                    variant_type = Some(RustEnumVariant::Tuple {
                        ty: rust_types::RustType::Simple { id: id.to_string() },
                        shared: RustEnumVariantShared {
                            id,
                            comments: Vec::new(),
                        },
                    });
                }
                _ => (),
            }
        }

        println!();
    }

    println!("{:#?}", data);

    let parsed_data = ParsedData::new();
    Ok(parsed_data)
}

fn parse_sortable_path(path: &[String]) -> Result<(Vec<Component>, Vec<Component>)> {
    let mut subject = path
        .iter()
        .map(|s| s.as_str().try_into())
        .collect::<Result<Vec<Component>>>()?;
    let child_depth = subject.last().map_or(1, |c| c.child_depth()) - 1;
    eprintln!("child_depth: {}", child_depth);
    let object = subject.split_off(subject.len() - child_depth);
    Ok((subject, object))
}

fn add_items<'c: 'p, 'p>(
    crate_: &'c Crate,
    item_processor: &'p mut ItemProcessor<'c>,
    trait_name: &'c str,
    filter: &'c [&'c str],
) {
    for root in find_roots(crate_, trait_name, filter) {
        let item = &crate_.index[root.parent];
        for id in root.assoc_types {
            let parent = PathComponent {
                item: NameableItem {
                    item,
                    overridden_name: None,
                    sorting_prefix: sorting_prefix(item),
                },
                type_: None,
                hide: false,
            };
            item_processor.add_to_work_queue(vec![parent], id);
        }
    }
}

struct Root<'a> {
    parent: &'a Id,
    assoc_types: Vec<&'a Id>,
}

fn find_roots<'a>(
    crate_: &'a Crate,
    trait_name: &'a str,
    filter: &'a [&'a str],
) -> impl Iterator<Item = Root<'a>> {
    crate_
        .index
        .iter()
        .filter_map(move |(parent, parent_item)| {
            if let ItemEnum::Impl(Impl {
                trait_: Some(Path { name, .. }),
                // for_: Type::ResolvedPath(_),
                items,
                ..
            }) = &parent_item.inner
            {
                if name.as_str() == trait_name {
                    let assoc_types = items
                        .iter()
                        .filter_map(|id| {
                            let item = &crate_.index[id];
                            item.name.as_deref().and_then(|name| {
                                if filter.contains(&name) {
                                    if let ItemEnum::AssocType {
                                        default: Some(Type::ResolvedPath(Path { id, .. })),
                                        ..
                                    } = &item.inner
                                    {
                                        Some(id)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                        })
                        .collect();
                    Some(Root {
                        parent,
                        assoc_types,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_sortable_path_for_enum() {
        let path = &["021-impl crux_core::App for shared::app::App", "007-Event"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        let (subject, object) = parse_sortable_path(path).unwrap();
        assert_eq!(
            subject,
            vec![
                Component {
                    type_: ComponentType::Namespace,
                    name: "shared::app::App".to_string()
                },
                Component {
                    type_: ComponentType::Enum,
                    name: "Event".to_string()
                }
            ]
        );
        assert_eq!(object, vec![]);
    }

    #[test]
    fn test_parse_sortable_path_for_variant() {
        let path = &[
            "021-impl crux_core::App for shared::app::App",
            "007-Event",
            "011-Decrement",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

        let (subject, object) = parse_sortable_path(path).unwrap();

        assert_eq!(
            subject,
            vec![
                Component {
                    type_: ComponentType::Namespace,
                    name: "shared::app::App".to_string()
                },
                Component {
                    type_: ComponentType::Enum,
                    name: "Event".to_string()
                }
            ]
        );
        assert_eq!(
            object,
            vec![Component {
                type_: ComponentType::Variant,
                name: "Decrement".to_string()
            }]
        );
    }

    #[test]
    fn test_parse_sortable_path_for_variant_tuple_field() {
        let path = &[
            "021-impl crux_core::core::effect::Effect for shared::app::Effect",
            "007-EffectFfi",
            "011-Http",
            "010-0",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

        let (subject, object) = parse_sortable_path(path).unwrap();

        assert_eq!(
            subject,
            vec![
                Component {
                    type_: ComponentType::Namespace,
                    name: "shared::app::Effect".to_string()
                },
                Component {
                    type_: ComponentType::Enum,
                    name: "EffectFfi".to_string()
                }
            ]
        );
        assert_eq!(
            object,
            vec![
                Component {
                    type_: ComponentType::Variant,
                    name: "Http".to_string()
                },
                Component {
                    type_: ComponentType::TupleVariantField,
                    name: "0".to_string()
                }
            ]
        );
    }
}
