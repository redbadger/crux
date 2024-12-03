use std::fs::File;

use pretty_assertions::assert_eq;
use rustdoc_types::{Crate, Generics, Id, Item, ItemEnum, Struct, StructKind, Visibility};

use super::*;
use crate::codegen::{parse, Registry};

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
    assert_eq!(name_of(&node), Some("Foo"));
}

#[test]
fn test_get_name_with_rename() {
    let name = Some("Foo".to_string());
    let attrs = vec![r#"#[serde(rename = "Bar")]"#.to_string()];
    let node = test_node(name, attrs);
    assert_eq!(name_of(&node), Some("Bar"));
}

#[test]
fn test_get_name_with_rename_no_whitespace() {
    let name = Some("Foo".to_string());
    let attrs = vec![r#"#[serde(rename="Bar")]"#.to_string()];
    let node = test_node(name, attrs);
    assert_eq!(name_of(&node), Some("Bar"));
}

#[test]
fn test_get_name_with_no_name() {
    let name = None;
    let attrs = vec![];
    let node = test_node(name, attrs);
    assert_eq!(name_of(&node), None);
}

#[test]
fn field_is_option_of_t() {
    static RUSTDOC: &'static [u8] = include_bytes!("fixtures/field_is_option_of_t.json");
    let crate_: Crate = serde_json::from_slice(RUSTDOC).unwrap();
    let nodes = parse(crate_);

    let mut prog = super::AscentProgram::default();
    prog.node = nodes;
    prog.run();

    // 345 (struct: ViewModel) to 343 (field: image)
    let mut out = prog.field_of;
    out.sort_by_key(|(name, _)| name.id.0);
    insta::assert_debug_snapshot!(&out, @r#"
    [
        (
            Node {
                id: Id(
                    345,
                ),
                item: Some(
                    Item {
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
                ),
                summary: None,
            },
            Node {
                id: Id(
                    343,
                ),
                item: Some(
                    Item {
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
                                    name: "Option",
                                    id: Id(
                                        173,
                                    ),
                                    args: Some(
                                        AngleBracketed {
                                            args: [
                                                Type(
                                                    ResolvedPath(
                                                        Path {
                                                            name: "CatImage",
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
                ),
                summary: None,
            },
        ),
    ]
    "#);

    // 343 (field: image) to 281 (struct: CatImage)
    let mut out = prog.type_for;
    out.sort_by_key(|(name, _)| name.id.0);
    insta::assert_debug_snapshot!(&out, @r#"
    [
        (
            Node {
                id: Id(
                    343,
                ),
                item: Some(
                    Item {
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
                                    name: "Option",
                                    id: Id(
                                        173,
                                    ),
                                    args: Some(
                                        AngleBracketed {
                                            args: [
                                                Type(
                                                    ResolvedPath(
                                                        Path {
                                                            name: "CatImage",
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
                ),
                summary: None,
            },
            Node {
                id: Id(
                    281,
                ),
                item: Some(
                    Item {
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
                ),
                summary: None,
            },
        ),
    ]
    "#);
}

#[test]
#[ignore = "not yet fully implemented"]
fn cat_facts_json() {
    static RUSTDOC: &'static [u8] = include_bytes!("fixtures/cat_facts/rustdoc.json");
    let crate_: Crate = serde_json::from_slice(RUSTDOC).unwrap();
    let nodes = parse(crate_);

    let actual = super::run(nodes);

    let writer = File::create("fixtures-cat_facts-actual.json").unwrap();
    serde_json::to_writer_pretty(writer, &actual).unwrap();
    let expected: Registry = serde_json::from_str(include_str!("fixtures/cat_facts/expected.json"))
        .expect("should deserialize");
    assert_eq!(actual, expected);
}

#[test]
#[ignore = "not yet fully implemented"]
fn notes_json() {
    static RUSTDOC: &'static [u8] = include_bytes!("fixtures/notes/rustdoc.json");
    let crate_: Crate = serde_json::from_slice(RUSTDOC).unwrap();
    let nodes = parse(crate_);

    let actual = super::run(nodes);

    let writer = File::create("fixtures-notes-actual.json").unwrap();
    serde_json::to_writer_pretty(writer, &actual).unwrap();
    let expected: Registry = serde_json::from_str(include_str!("fixtures/notes/expected.json"))
        .expect("should deserialize");
    assert_eq!(actual, expected);
}

#[test]
fn cat_facts() {
    static RUSTDOC: &'static [u8] = include_bytes!("fixtures/cat_facts/rustdoc.json");
    let crate_: Crate = serde_json::from_slice(RUSTDOC).unwrap();
    let nodes = parse(crate_);

    let registry = super::run(nodes);

    insta::assert_debug_snapshot!(&registry, @r#"
    {
        "CatImage": Struct(
            [
                Named {
                    name: "href",
                    value: Str,
                },
            ],
        ),
        "Effect": Enum(
            {
                0: Named {
                    name: "Http",
                    value: NewType(
                        TypeName(
                            "Operation",
                        ),
                    ),
                },
                1: Named {
                    name: "KeyValue",
                    value: NewType(
                        TypeName(
                            "Operation",
                        ),
                    ),
                },
                2: Named {
                    name: "Platform",
                    value: NewType(
                        TypeName(
                            "Operation",
                        ),
                    ),
                },
                3: Named {
                    name: "Render",
                    value: NewType(
                        TypeName(
                            "Operation",
                        ),
                    ),
                },
                4: Named {
                    name: "Time",
                    value: NewType(
                        TypeName(
                            "Operation",
                        ),
                    ),
                },
            },
        ),
        "Event": Enum(
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
        "ViewModel": Struct(
            [
                Named {
                    name: "fact",
                    value: Str,
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
                    value: Str,
                },
            ],
        ),
    }
    "#);
}

#[test]
fn bridge_echo() {
    static RUSTDOC: &'static [u8] = include_bytes!("fixtures/bridge_echo_rustdoc.json");
    let crate_: Crate = serde_json::from_slice(RUSTDOC).unwrap();
    let nodes = parse(crate_);

    let containers = super::run(nodes);

    insta::assert_debug_snapshot!(&containers, @r#"
    {
        "Effect": Enum(
            {
                0: Named {
                    name: "Render",
                    value: NewType(
                        TypeName(
                            "Operation",
                        ),
                    ),
                },
            },
        ),
        "Event": Enum(
            {
                0: Named {
                    name: "Tick",
                    value: Unit,
                },
                1: Named {
                    name: "NewPeriod",
                    value: Unit,
                },
            },
        ),
        "ViewModel": Struct(
            [
                Named {
                    name: "count",
                    value: U64,
                },
            ],
        ),
    }
    "#);
}
