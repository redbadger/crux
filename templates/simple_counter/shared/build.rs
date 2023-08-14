fn main() {
    uniffi::generate_scaffolding("./src/{{name}}.udl").unwrap();
}
