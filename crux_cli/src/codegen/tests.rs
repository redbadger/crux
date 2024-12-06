use std::fs::File;

use rustdoc_types::Crate;

use crate::codegen::Registry;

#[test]
#[ignore = "not yet fully implemented"]
fn cat_facts_json() {
    static RUSTDOC: &'static [u8] = include_bytes!("fixtures/cat_facts/rustdoc.json");
    let crate_: Crate = serde_json::from_slice(RUSTDOC).unwrap();
    let actual = super::run(crate_);

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
    let actual = super::run(crate_);

    let writer = File::create("fixtures-notes-actual.json").unwrap();
    serde_json::to_writer_pretty(writer, &actual).unwrap();
    let expected: Registry = serde_json::from_str(include_str!("fixtures/notes/expected.json"))
        .expect("should deserialize");
    assert_eq!(actual, expected);
}

#[test]
fn bridge_echo() {
    static RUSTDOC: &'static [u8] = include_bytes!("fixtures/bridge_echo_rustdoc.json");
    let crate_: Crate = serde_json::from_slice(RUSTDOC).unwrap();
    let containers = super::run(crate_);

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

#[test]
fn cat_facts() {
    static RUSTDOC: &'static [u8] = include_bytes!("fixtures/cat_facts/rustdoc.json");
    let crate_: Crate = serde_json::from_slice(RUSTDOC).unwrap();

    let registry = super::run(crate_);

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
