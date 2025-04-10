use anyhow::Result;
use ascent::ascent;
use log::{debug, info};
use rustdoc_types::Crate;

use super::item::*;
use super::node::{CrateNode, ItemNode, SummaryNode};

ascent! {
    #![measure_rule_times]
    pub struct Filter;

    // ------- facts ------------------
    relation item(ItemNode);
    relation summary(SummaryNode);
    relation ext_crate(CrateNode);

    // ------- rules ------------------

    relation has_summary(ItemNode, SummaryNode);
    has_summary(i, s) <-- item(i), summary(s), if i.has_summary(s);

    relation is_struct(ItemNode);
    is_struct(s) <-- item(s) if is_struct(&s.item);

    relation is_enum(ItemNode);
    is_enum(e) <-- item(e) if is_enum(&e.item);

    relation variant(ItemNode, ItemNode);
    variant(e, v) <-- is_enum(e), item(v) if e.has_variant(v);

    relation field(ItemNode, ItemNode);
    field(s, f) <-- is_struct(s), item(f) if s.has_field(f);
    field(v, f) <-- variant(e, v), item(f) if v.has_field(f);

    relation local_type_of(ItemNode, ItemNode);
    local_type_of(f, t) <-- item(f), item(t) if f.is_of_local_type(t);

    relation remote_type_of(ItemNode, SummaryNode);
    remote_type_of(f, t) <-- item(f), summary(t) if f.is_of_remote_type(t);

    // app structs have an implementation of the App trait
    relation app(ItemNode, ItemNode);
    app(imp, app) <--
        is_struct(app),
        item(imp),
        if imp.is_impl_for(app, "App");

    // app hierarchy
    relation parent(ItemNode, ItemNode);
    parent(parent, child) <--
        app(_, parent),
        app(_, child),
        field(parent, field),
        local_type_of(field, child);

    relation root_app(ItemNode, ItemNode);
    root_app(impl_, app) <--
        app(impl_, app),
        !parent(_, app);

    // a view model is an associated type of an app
    relation view_model(ItemNode, ItemNode);
    view_model(app, view_model) <--
        root_app(impl_, app),
        local_type_of(item, view_model),
        if impl_.has_associated_item(item, "ViewModel");

    // an event is an associated type of an app
    relation event(ItemNode, ItemNode);
    event(app, event) <--
        root_app(impl_, app),
        local_type_of(item, event),
        if impl_.has_associated_item(item, "Event");

    // effect enums have an implementation of the Effect trait
    // and an associated Ffi type, which is the FFI representation of the effect
    relation effect(ItemNode, ItemNode);
    effect(app, effect_ffi) <--
        root_app(app_impl, app),
        is_enum(effect),
        item(effect_impl),
        if effect_impl.is_impl_for(effect, "Effect"),
        has_summary(app, app_summary),
        has_summary(effect, effect_summary),
        if app_summary.in_same_module_as(effect_summary),
        local_type_of(effect_ffi_item, effect_ffi),
        if effect_impl.has_associated_item(effect_ffi_item, "Ffi");

    // operation is a struct or enum that implements the Operation trait
    relation operation(ItemNode, ItemNode);
    operation(op_impl, op) <--
        is_struct(op),
        item(op_impl),
        if op_impl.is_impl_for(op, "Operation");
    operation(op_impl, op) <--
        is_enum(op),
        item(op_impl),
        if op_impl.is_impl_for(op, "Operation");

    // Output is an associated type of an impl of the Operation trait
    relation output(ItemNode);
    output(out) <--
        operation(op_impl, op),
        local_type_of(item, out),
        if op_impl.has_associated_item(item, "Output");

    relation root(ItemNode);
    root(x) <-- view_model(app, x);
    root(x) <-- event(app, x);
    root(x) <-- effect(app, x);
    root(x) <-- operation(op_impl, x);
    root(x) <-- output(x);

    // set of all the edges we are interested in
    relation edge(ItemNode, ItemNode);

    // roots that are unit structs
    edge(root, root) <--
        root(root),
        is_struct(root), if is_struct_unit(&root.item);
    // roots that have fields
    edge(root, field) <--
        root(root),
        field(root, field);
    // roots that have variants
    edge(root, variant) <--
        root(root),
        variant(root, variant);

    edge(type_, field) <--
        edge(_, type_),
        field(type_, field);
    edge(type_, variant) <--
        edge(_, type_),
        variant(type_, variant);

    edge(field, type_) <--
        edge(_, field),
        local_type_of(field, type_);

    relation crates(String);
    crates(n) <--
        ext_crate(c),
        edge(a, b),
        remote_type_of(b, s),
        if s.points_to_crate(c),
        let n = &c.crate_.name;
}

impl Filter {
    pub fn process(&mut self, crate_name: &str, crate_: &Crate) -> Result<()> {
        info!("Updating filter for {}", crate_name);
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
            .filter_map(|item| {
                if is_relevant(item) {
                    Some((ItemNode::new(crate_name.to_string(), item.clone()),))
                } else {
                    None
                }
            })
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

    pub fn get_crates(&self) -> Vec<String> {
        self.crates.iter().map(|(crate_,)| crate_.clone()).collect()
    }
}

#[cfg(test)]
#[path = "filter_tests.rs"]
mod filter_tests;
