use anyhow::Result;
use ascent::ascent;
use log::{debug, info};
use rustdoc_types::Crate;

use crate::codegen::node::SummaryNode;

use super::node::{CrateNode, Edge, GlobalId, ItemNode};

ascent! {
    pub struct Graph;

    // facts
    relation start(GlobalId);
    relation item(ItemNode);
    relation summary(SummaryNode);
    relation ext_crate(CrateNode);

    // rules
    relation edge(GlobalId, GlobalId, Edge);
    edge(a, b, e) <--
        (start(a) || edge(_, a, _)),
        item(i) if i.id.id == a.id,
        for (b, e) in i.edges();

    relation krate(GlobalId, String);
    krate(id, n) <--
        ext_crate(c),
        edge(_, id, _),
        summary(s) if s.id.id == id.id,
        if s.points_to_crate(c),
        let n = &c.crate_.name;
}

impl Graph {
    pub fn process(&mut self, crate_name: &str, crate_: &Crate) -> Result<()> {
        info!("Updating graph for {}", crate_name);
        let start = GlobalId {
            crate_: crate_name.to_string(),
            id: crate_.root.0,
        };
        self.start = vec![(start,)];
        self.summary = crate_
            .paths
            .iter()
            .map(|(id, summary)| {
                (SummaryNode::new(
                    crate_name.to_string(),
                    id.0,
                    summary.clone(),
                ),)
            })
            .collect::<Vec<_>>();
        self.item = crate_
            .index
            .values()
            .map(|item| (ItemNode::new(crate_name.to_string(), item.clone()),))
            .collect::<Vec<_>>();
        self.ext_crate = crate_
            .external_crates
            .iter()
            .map(|(id, crate_)| (CrateNode::new(crate_name.to_string(), *id, crate_.clone()),))
            .collect::<Vec<_>>();

        self.run();
        debug!("{}", self.scc_times_summary());

        Ok(())
    }
}
