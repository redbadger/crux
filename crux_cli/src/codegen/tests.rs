use anyhow::Result;
use pretty_assertions::assert_eq;
use rstest::rstest;
use rustdoc_types::Crate;

use crate::codegen::Registry;

fn load_rustdoc(name: &str) -> Result<Crate> {
    Ok(match name {
        "counter" => serde_json::from_slice(include_bytes!("fixtures/counter/rustdoc.json"))?,
        "counter_http" => {
            serde_json::from_slice(include_bytes!("fixtures/counter_http/rustdoc.json"))?
        }
        "notes" => serde_json::from_slice(include_bytes!("fixtures/notes/rustdoc.json"))?,
        "crux_core" => serde_json::from_slice(include_bytes!("fixtures/crux_core.json"))?,
        "crux_http" => serde_json::from_slice(include_bytes!("fixtures/crux_http.json"))?,
        "crux_kv" => serde_json::from_slice(include_bytes!("fixtures/crux_kv.json"))?,
        "crux_platform" => serde_json::from_slice(include_bytes!("fixtures/crux_platform.json"))?,
        "crux_time" => serde_json::from_slice(include_bytes!("fixtures/crux_time.json"))?,
        _ => panic!("unknown crate {name}"),
    })
}

fn load_expected(name: &str) -> Result<Registry> {
    Ok(match name {
        "counter" => serde_json::from_slice(include_bytes!("fixtures/counter/expected.json"))?,
        "counter_http" => {
            serde_json::from_slice(include_bytes!("fixtures/counter_http/expected.json"))?
        }
        "notes" => serde_json::from_slice(include_bytes!("fixtures/notes/expected.json"))?,
        _ => panic!("unknown example {name}"),
    })
}

#[rstest]
#[case::counter("counter")]
#[case::counter_http("counter_http")]
#[case::notes("notes")]
fn full(#[case] example: &str) {
    let actual = super::run(example, load_rustdoc).unwrap();
    let expected: Registry = load_expected(example).expect("should deserialize");

    assert_eq!(actual, expected);
}
