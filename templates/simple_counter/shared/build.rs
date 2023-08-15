fn main() {
    uniffi::generate_scaffolding("./src/{{core_name_dashes}}.udl").unwrap();
}
