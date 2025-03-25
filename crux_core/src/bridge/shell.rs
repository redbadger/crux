pub trait Shell: Send + Sync {
    fn handle_effects(&self, requests: Vec<u8>);
}
