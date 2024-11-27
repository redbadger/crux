use std::hash::Hash;

use rustdoc_types::{Id, Item, ItemSummary};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: Id,
    pub item: Option<Item>,
    pub summary: Option<ItemSummary>,
}

impl Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
