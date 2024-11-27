use rustdoc_types::Crate;

use crate::codegen::data::Node;

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

    let containers = super::run(nodes);
    insta::assert_debug_snapshot!(&containers, @r#"
    [
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
            "Effect",
            Enum(
                {
                    0: Named {
                        name: "Render",
                        value: Tuple(
                            [],
                        ),
                    },
                    1: Named {
                        name: "Http",
                        value: Tuple(
                            [],
                        ),
                    },
                    2: Named {
                        name: "Time",
                        value: Tuple(
                            [],
                        ),
                    },
                    3: Named {
                        name: "KeyValue",
                        value: Tuple(
                            [],
                        ),
                    },
                    4: Named {
                        name: "Platform",
                        value: Tuple(
                            [],
                        ),
                    },
                },
            ),
        ),
    ]
    "#);
}
