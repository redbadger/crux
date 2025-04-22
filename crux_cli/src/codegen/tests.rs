use anyhow::Result;
use pretty_assertions::assert_eq;
use rstest::rstest;
use rustdoc_types::Crate;

use crate::codegen::Registry;

fn load_rustdoc(name: &str) -> Result<Crate> {
    Ok(match name {
        "bridge_echo" => {
            serde_json::from_slice(include_bytes!("fixtures/bridge_echo/rustdoc.json"))?
        }
        "cat_facts" => serde_json::from_slice(include_bytes!("fixtures/cat_facts/rustdoc.json"))?,
        "counter" => serde_json::from_slice(include_bytes!("fixtures/counter/rustdoc.json"))?,
        "hello_world" => {
            serde_json::from_slice(include_bytes!("fixtures/hello_world/rustdoc.json"))?
        }
        "notes" => serde_json::from_slice(include_bytes!("fixtures/notes/rustdoc.json"))?,
        "simple_counter" => {
            serde_json::from_slice(include_bytes!("fixtures/simple_counter/rustdoc.json"))?
        }
        "tap_to_pay" => serde_json::from_slice(include_bytes!("fixtures/tap_to_pay/rustdoc.json"))?,
        "crux_core" => serde_json::from_slice(include_bytes!("fixtures/crux_core.json"))?,
        "crux_http" => serde_json::from_slice(include_bytes!("fixtures/crux_http.json"))?,
        "crux_kv" => serde_json::from_slice(include_bytes!("fixtures/crux_kv.json"))?,
        "crux_platform" => serde_json::from_slice(include_bytes!("fixtures/crux_platform.json"))?,
        "crux_time" => serde_json::from_slice(include_bytes!("fixtures/crux_time.json"))?,
        _ => panic!("unknown crate {}", name),
    })
}

fn load_expected(name: &str) -> Result<Registry> {
    Ok(match name {
        "bridge_echo" => {
            serde_json::from_slice(include_bytes!("fixtures/bridge_echo/expected.json"))?
        }
        "cat_facts" => serde_json::from_slice(include_bytes!("fixtures/cat_facts/expected.json"))?,
        "counter" => serde_json::from_slice(include_bytes!("fixtures/counter/expected.json"))?,
        "hello_world" => {
            serde_json::from_slice(include_bytes!("fixtures/hello_world/expected.json"))?
        }
        "notes" => serde_json::from_slice(include_bytes!("fixtures/notes/expected.json"))?,
        "simple_counter" => {
            serde_json::from_slice(include_bytes!("fixtures/simple_counter/expected.json"))?
        }
        "tap_to_pay" => {
            serde_json::from_slice(include_bytes!("fixtures/tap_to_pay/expected.json"))?
        }
        _ => panic!("unknown example {}", name),
    })
}

#[rstest]
#[case::bridge_echo("bridge_echo")]
#[case::cat_facts("cat_facts")]
#[case::counter("counter")]
#[case::hello_world("hello_world")]
#[case::notes("notes")]
#[case::simple_counter("simple_counter")]
#[case::tap_to_pay("tap_to_pay")]
fn full(#[case] example: &str) {
    let actual = super::run(example, load_rustdoc).unwrap();
    let expected: Registry = load_expected(example).expect("should deserialize");

    assert_eq!(actual, expected);
}
