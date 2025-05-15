use rustdoc_types::Crate;

use crate::codegen::{filter::Filter, node::ItemNode};

#[test]
fn field_is_option_of_t() {
    static RUSTDOC: &[u8] = include_bytes!("fixtures/field_is_option_of_t.json");
    let crate_: Crate = serde_json::from_slice(RUSTDOC).unwrap();

    let mut prog = Filter {
        item: crate_
            .index
            .values()
            .map(|item| (ItemNode::new("test".to_string(), item.clone()),))
            .collect::<Vec<_>>(),
        ..Default::default()
    };
    prog.run();

    // 345 (struct: ViewModel) to 343 (field: image)
    let mut out = prog.field;
    out.sort_by_key(|(node, _)| node.item.id.0);
    insta::assert_debug_snapshot!(&out, @r#"
    [
        (
            ItemNode {
                id: GlobalId {
                    crate_: "test",
                    id: 345,
                },
                item: Item {
                    id: Id(
                        345,
                    ),
                    crate_id: 0,
                    name: Some(
                        "ViewModel",
                    ),
                    span: Some(
                        Span {
                            filename: "shared/src/app.rs",
                            begin: (
                                54,
                                0,
                            ),
                            end: (
                                58,
                                1,
                            ),
                        },
                    ),
                    visibility: Public,
                    docs: None,
                    links: {},
                    attrs: [],
                    deprecation: None,
                    inner: Struct(
                        Struct {
                            kind: Plain {
                                fields: [
                                    Id(
                                        343,
                                    ),
                                ],
                                has_stripped_fields: false,
                            },
                            generics: Generics {
                                params: [],
                                where_predicates: [],
                            },
                            impls: [],
                        },
                    ),
                },
            },
            ItemNode {
                id: GlobalId {
                    crate_: "test",
                    id: 343,
                },
                item: Item {
                    id: Id(
                        343,
                    ),
                    crate_id: 0,
                    name: Some(
                        "image",
                    ),
                    span: Some(
                        Span {
                            filename: "shared/src/app.rs",
                            begin: (
                                56,
                                4,
                            ),
                            end: (
                                56,
                                31,
                            ),
                        },
                    ),
                    visibility: Public,
                    docs: None,
                    links: {},
                    attrs: [],
                    deprecation: None,
                    inner: StructField(
                        ResolvedPath(
                            Path {
                                path: "Option",
                                id: Id(
                                    173,
                                ),
                                args: Some(
                                    AngleBracketed {
                                        args: [
                                            Type(
                                                ResolvedPath(
                                                    Path {
                                                        path: "CatImage",
                                                        id: Id(
                                                            281,
                                                        ),
                                                        args: Some(
                                                            AngleBracketed {
                                                                args: [],
                                                                constraints: [],
                                                            },
                                                        ),
                                                    },
                                                ),
                                            ),
                                        ],
                                        constraints: [],
                                    },
                                ),
                            },
                        ),
                    ),
                },
            },
        ),
    ]
    "#);

    // 343 (field: image) to 281 (struct: CatImage)
    let mut out = prog.local_type_of;
    out.sort_by_key(|(node, _)| node.item.id.0);
    insta::assert_debug_snapshot!(&out, @r#"
    [
        (
            ItemNode {
                id: GlobalId {
                    crate_: "test",
                    id: 343,
                },
                item: Item {
                    id: Id(
                        343,
                    ),
                    crate_id: 0,
                    name: Some(
                        "image",
                    ),
                    span: Some(
                        Span {
                            filename: "shared/src/app.rs",
                            begin: (
                                56,
                                4,
                            ),
                            end: (
                                56,
                                31,
                            ),
                        },
                    ),
                    visibility: Public,
                    docs: None,
                    links: {},
                    attrs: [],
                    deprecation: None,
                    inner: StructField(
                        ResolvedPath(
                            Path {
                                path: "Option",
                                id: Id(
                                    173,
                                ),
                                args: Some(
                                    AngleBracketed {
                                        args: [
                                            Type(
                                                ResolvedPath(
                                                    Path {
                                                        path: "CatImage",
                                                        id: Id(
                                                            281,
                                                        ),
                                                        args: Some(
                                                            AngleBracketed {
                                                                args: [],
                                                                constraints: [],
                                                            },
                                                        ),
                                                    },
                                                ),
                                            ),
                                        ],
                                        constraints: [],
                                    },
                                ),
                            },
                        ),
                    ),
                },
            },
            ItemNode {
                id: GlobalId {
                    crate_: "test",
                    id: 281,
                },
                item: Item {
                    id: Id(
                        281,
                    ),
                    crate_id: 0,
                    name: Some(
                        "CatImage",
                    ),
                    span: Some(
                        Span {
                            filename: "shared/src/app.rs",
                            begin: (
                                41,
                                0,
                            ),
                            end: (
                                43,
                                1,
                            ),
                        },
                    ),
                    visibility: Public,
                    docs: None,
                    links: {},
                    attrs: [],
                    deprecation: None,
                    inner: Struct(
                        Struct {
                            kind: Plain {
                                fields: [
                                    Id(
                                        308,
                                    ),
                                ],
                                has_stripped_fields: false,
                            },
                            generics: Generics {
                                params: [],
                                where_predicates: [],
                            },
                            impls: [
                                Id(
                                    309,
                                ),
                                Id(
                                    310,
                                ),
                                Id(
                                    311,
                                ),
                                Id(
                                    312,
                                ),
                                Id(
                                    313,
                                ),
                                Id(
                                    314,
                                ),
                                Id(
                                    315,
                                ),
                                Id(
                                    316,
                                ),
                                Id(
                                    317,
                                ),
                                Id(
                                    318,
                                ),
                                Id(
                                    319,
                                ),
                                Id(
                                    320,
                                ),
                                Id(
                                    321,
                                ),
                                Id(
                                    322,
                                ),
                                Id(
                                    323,
                                ),
                                Id(
                                    324,
                                ),
                                Id(
                                    325,
                                ),
                                Id(
                                    326,
                                ),
                                Id(
                                    327,
                                ),
                                Id(
                                    329,
                                ),
                                Id(
                                    331,
                                ),
                                Id(
                                    332,
                                ),
                                Id(
                                    334,
                                ),
                                Id(
                                    335,
                                ),
                                Id(
                                    337,
                                ),
                                Id(
                                    339,
                                ),
                                Id(
                                    341,
                                ),
                            ],
                        },
                    ),
                },
            },
        ),
    ]
    "#);
}
