use anyhow::Result;
use ascent::ascent;
use log::{debug, info};
use rustdoc_types::Crate;

use super::node::{Edge, GlobalId, ItemNode};

ascent! {
    pub struct Graph;

    // facts
    relation start(GlobalId);
    relation item(ItemNode);

    // rules
    relation edge(GlobalId, GlobalId, Edge);
    edge(a, b, e) <--
        start(a),
        item(i) if i.id.id == a.id,
        for (b, e) in i.edges();
    edge(a, b, e) <--
        edge(_, a, _),
        item(i) if i.id.id == a.id,
        for (b, e) in i.edges();

}

impl Graph {
    pub fn process(&mut self, crate_name: &str, crate_: &Crate) -> Result<()> {
        info!("Updating graph for {}", crate_name);
        let start = GlobalId {
            crate_: crate_name.to_string(),
            id: crate_.root.0,
        };
        self.start = vec![(start,)];
        self.item = crate_
            .index
            .values()
            .map(|item| (ItemNode::new(crate_name.to_string(), item.clone()),))
            .collect::<Vec<_>>();

        self.run();
        debug!("{}", self.scc_times_summary());

        Ok(())
    }
}
