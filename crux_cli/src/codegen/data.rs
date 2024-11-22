use std::collections::HashMap;

use rustdoc_types::{Crate, Id, Item, ItemSummary};
use serde::Serialize;

pub struct Data {
    pub crate_: Crate,
    pub nodes_by_id: HashMap<Id, Node>,
}

impl Data {
    pub fn new(crate_: Crate) -> Self {
        let mut nodes_by_id = HashMap::new();

        // items
        for (id, item) in &crate_.index {
            nodes_by_id
                .entry(*id)
                .or_insert_with(|| Node::new(*id))
                .item = Some(item.clone());
        }

        // paths
        for (id, path) in &crate_.paths {
            nodes_by_id
                .entry(*id)
                .or_insert_with(|| Node::new(*id))
                .summary = Some(path.clone());
        }

        Self {
            crate_,
            nodes_by_id,
        }
    }

    pub fn node_by_id(&self, id: &Id) -> Option<&Node> {
        let node = self
            .nodes_by_id
            .get(id)
            .expect("node should exist for all items and paths");
        let skip = match &node.item {
            Some(x) => x.attrs.contains(&"#[serde(skip)]".to_string()),
            _ => false,
        };
        (!skip).then_some(node)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Node {
    pub id: Id,
    pub item: Option<Item>,
    pub summary: Option<ItemSummary>,
}

impl Node {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            item: None,
            summary: None,
        }
    }
}

impl std::hash::Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
