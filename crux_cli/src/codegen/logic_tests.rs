use crate::codegen::data::{Edge, Node};

#[test]
fn cat_facts() {
    static FILE: &str = include_str!("fixtures/cat_facts.json");
    let data: Vec<(Node, Node, Edge)> = serde_json::from_str(FILE).unwrap();

    let containers = super::run(data);
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
