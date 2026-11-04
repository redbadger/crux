//! The protocol of `crux_kv` follows the `facet_typegen` feature of
//! `crux_core`.
//!
//! An app turns on `crux_core/facet_typegen` and no feature of `crux_kv`.
//! This test has the same features: the dev-dependency on `crux_core` turns
//! on `facet_typegen`, and the test does not need the `facet_typegen`
//! feature of `crux_kv`. The operation must still register the protocol.

use std::fs;

use crux_core::{
    capability::Operation as _,
    type_generation::facet::{Config, TypeRegistry},
};
use crux_kv::operation::GetValue;

#[test]
fn an_operation_registers_the_protocol_without_a_feature_of_the_capability() {
    let mut registry = TypeRegistry::new();
    GetValue::register_types_facet(&mut registry).expect("should register the operation");
    let generator = registry.build().expect("should build the registry");

    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator
        .swift(&Config::builder("App", dir.path()).build())
        .expect("swift type generation should succeed");

    let protocol = fs::read_to_string(dir.path().join("App/Sources/App/KeyValue.swift"))
        .expect("should write the protocol beside the module");
    assert!(
        protocol.contains("protocol KeyValueHandler"),
        "unexpected protocol:\n{protocol}"
    );
}
