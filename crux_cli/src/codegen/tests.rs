use std::fs::File;

use anyhow::Result;
use pretty_assertions::assert_eq;
use rustdoc_types::Crate;

use crate::codegen::Registry;

fn load(name: String) -> Result<Crate> {
    Ok(match name.as_str() {
        "cat_facts" => serde_json::from_str(include_str!("fixtures/cat_facts/rustdoc.json"))?,
        "counter" => serde_json::from_str(include_str!("fixtures/counter/rustdoc.json"))?,
        "notes" => serde_json::from_str(include_str!("fixtures/notes/rustdoc.json"))?,
        "crux_core" => serde_json::from_str(include_str!("fixtures/crux_core.json"))?,
        "crux_http" => serde_json::from_str(include_str!("fixtures/crux_http.json"))?,
        "crux_kv" => serde_json::from_str(include_str!("fixtures/crux_kv.json"))?,
        "crux_platform" => serde_json::from_str(include_str!("fixtures/crux_platform.json"))?,
        "crux_time" => serde_json::from_str(include_str!("fixtures/crux_time.json"))?,
        _ => panic!("unknown crate {}", name),
    })
}

#[test]
#[ignore = "not yet fully implemented"]
fn cat_facts_json() {
    let actual = super::run("cat_facts", true, load).unwrap();

    let writer = File::create("fixtures-cat_facts-actual.json").unwrap();
    serde_json::to_writer_pretty(writer, &actual).unwrap();
    let expected: Registry = serde_json::from_str(include_str!("fixtures/cat_facts/expected.json"))
        .expect("should deserialize");
    assert_eq!(actual, expected);
}

#[test]
#[ignore = "not yet fully implemented"]
fn counter_json() {
    let actual = super::run("counter", true, load).unwrap();

    let writer = File::create("fixtures-counter-actual.json").unwrap();
    serde_json::to_writer_pretty(writer, &actual).unwrap();
    let expected: Registry = serde_json::from_str(include_str!("fixtures/counter/expected.json"))
        .expect("should deserialize");
    assert_eq!(actual, expected);
}

#[test]
#[ignore = "not yet fully implemented"]
fn notes_json() {
    let actual = super::run("notes", true, load).unwrap();

    let writer = File::create("fixtures-notes-actual.json").unwrap();
    serde_json::to_writer_pretty(writer, &actual).unwrap();
    let expected: Registry = serde_json::from_str(include_str!("fixtures/notes/expected.json"))
        .expect("should deserialize");
    assert_eq!(actual, expected);
}

#[test]
fn bridge_echo() {
    static RUSTDOC: &'static [u8] = include_bytes!("fixtures/bridge_echo_rustdoc.json");
    let actual = super::run("bridge_echo", false, |_| {
        Ok(serde_json::from_slice(RUSTDOC).unwrap())
    })
    .unwrap();

    insta::assert_debug_snapshot!(&actual, @r#"
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
    let actual = super::run("cat_facts", false, |_| {
        Ok(serde_json::from_slice(RUSTDOC).unwrap())
    })
    .unwrap();

    insta::assert_debug_snapshot!(&actual, @r#"
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
