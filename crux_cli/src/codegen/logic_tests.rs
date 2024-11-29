use rustdoc_types::Crate;

use crate::codegen::logic::Node;

#[test]
fn cat_facts() {
    static RUSTDOC: &'static [u8] = include_bytes!("fixtures/cat_facts_rustdoc.json");
    let crate_: Crate = serde_json::from_slice(RUSTDOC).unwrap();
    let nodes = crate_
        .index
        .values()
        .flat_map(|item| {
            if item.attrs.contains(&"#[serde(skip)]".to_string()) {
                None
            } else {
                Some((Node {
                    id: item.id,
                    item: Some(item.clone()),
                    summary: crate_.paths.get(&item.id).cloned(),
                },))
            }
        })
        .collect::<Vec<_>>();

    let mut containers = super::run(nodes);
    containers.sort_by_key(|(name, _)| name.to_string());

    insta::assert_debug_snapshot!(&containers, @r#"
    [
        (
            "CatImage",
            Struct(
                [
                    Named {
                        name: "href",
                        value: TypeName(
                            "String",
                        ),
                    },
                ],
            ),
        ),
        (
            "Effect",
            Enum(
                {
                    0: Named {
                        name: "Http",
                        value: Tuple(
                            [],
                        ),
                    },
                    1: Named {
                        name: "KeyValue",
                        value: Tuple(
                            [],
                        ),
                    },
                    2: Named {
                        name: "Platform",
                        value: Tuple(
                            [],
                        ),
                    },
                    3: Named {
                        name: "Render",
                        value: Tuple(
                            [],
                        ),
                    },
                    4: Named {
                        name: "Time",
                        value: Tuple(
                            [],
                        ),
                    },
                },
            ),
        ),
        (
            "Event",
            Enum(
                {
                    0: Named {
                        name: "None",
                        value: Unit,
                    },
                    1: Named {
                        name: "Clear",
                        value: Unit,
                    },
                    2: Named {
                        name: "Get",
                        value: Unit,
                    },
                    3: Named {
                        name: "Fetch",
                        value: Unit,
                    },
                    4: Named {
                        name: "GetPlatform",
                        value: Unit,
                    },
                    5: Named {
                        name: "Restore",
                        value: Unit,
                    },
                },
            ),
        ),
        (
            "ViewModel",
            Struct(
                [
                    Named {
                        name: "fact",
                        value: TypeName(
                            "String",
                        ),
                    },
                    Named {
                        name: "image",
                        value: Option(
                            TypeName(
                                "CatImage",
                            ),
                        ),
                    },
                    Named {
                        name: "platform",
                        value: TypeName(
                            "String",
                        ),
                    },
                ],
            ),
        ),
    ]
    "#);
}
