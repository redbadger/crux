#![allow(clippy::no_effect_underscore_binding)]

use ascent::ascent;
use log::{debug, info};
use rustdoc_types::{Crate, Item, ItemEnum, Id, Struct, StructKind, Generics, Visibility};
use std::collections::HashMap;

use super::item::{is_enum, is_relevant, is_struct, is_struct_unit};
use super::node::{CrateNode, ItemNode, SummaryNode};

/// Standard library crate names that should be skipped during type generation
pub const STD_CRATES: &[&str] = &["std", "core", "alloc", "proc_macro", "test"];

/// Check if a type is from the standard library and shouldn't be generated
fn is_std_type(item: &ItemNode) -> bool {
    STD_CRATES.contains(&item.id.crate_.as_str())
}

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

    // Track external types that need to be resolved
    relation external_type_needed(SummaryNode);
    external_type_needed(t) <--
        remote_type_of(_, t),
        !has_summary(_, t);

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
    root(x) <-- view_model(app, x), if !is_std_type(x);
    root(x) <-- event(app, x), if !is_std_type(x);
    root(x) <-- effect(app, x), if !is_std_type(x);
    root(x) <-- operation(op_impl, x), if !is_std_type(x);
    root(x) <-- output(x), if !is_std_type(x);
    
    // Add external types that are resolved as roots
    root(x) <--
        external_type_needed(summary),
        item(x),
        has_summary(x, summary),
        if !is_std_type(x);

    // Note: workspace crate detection happens dynamically in the processing phase

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
    
    // Create edges for remote types that resolve to actual items
    // This connects workspace crates like 'models' to the main app hierarchy
    edge(field, type_) <--
        edge(_, field),
        remote_type_of(field, summary),
        item(type_),
        has_summary(type_, summary);

    relation crates(String);
    crates(n) <--
        ext_crate(c),
        edge(a, b),
        remote_type_of(b, s),
        if s.points_to_crate(c),
        let n = &c.crate_.name;
}

impl Filter {
    pub fn process(&mut self, crate_name: &str, crate_: &Crate) {
        info!("Updating filter for {crate_name}");
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
    }

    pub fn get_crates(&self) -> Vec<String> {
        self.crates.iter().map(|(crate_,)| crate_.clone()).collect()
    }

    pub fn get_external_types(&self) -> Vec<SummaryNode> {
        self.external_type_needed
            .iter()
            .map(|(summary,)| summary.clone())
            .collect()
    }


    /// Add workspace external types as synthetic edges to ensure they become containers
    pub fn add_workspace_external_types(&mut self, workspace_external_types: Vec<SummaryNode>) {
        for workspace_type in workspace_external_types {
            if let (Some(actual_crate), Some(type_name)) = (workspace_type.actual_crate_name(), extract_type_name(&workspace_type)) {
                // Skip standard library types
                if STD_CRATES.contains(&actual_crate.as_str()) {
                    continue;
                }
                
                let synthetic_item = create_synthetic_item(&type_name, &workspace_type);
                let synthetic_node = ItemNode::new(actual_crate, synthetic_item);
                
                if is_std_type(&synthetic_node) {
                    continue;
                }
                
                self.item.push((synthetic_node.clone(),));
                self.root.push((synthetic_node.clone(),));
            }
        }
        
        // Run the filter again to process the synthetic types
        self.run();
    }

    /// Add all public types (structs/enums) from a crate as roots
    /// This ensures comprehensive type generation for frontend bindings
    pub fn add_all_public_types_as_roots(&mut self, crate_name: &str) {
        // Find all structs and enums from this crate, excluding standard library types
        let items_to_add: Vec<ItemNode> = self.item.iter()
            .filter(|(item,)| item.id.crate_ == crate_name)
            .filter(|(item,)| is_struct(&item.item) || is_enum(&item.item))
            .filter(|(item,)| !is_std_type(item))
            .map(|(item,)| item.clone())
            .collect();
            
        for item in items_to_add {
            if item.name().is_some() {
                self.root.push((item,));
            }
        }
        
        self.run();
    }
}

/// Extract the type name from a `SummaryNode` path
fn extract_type_name(summary: &SummaryNode) -> Option<String> {
    if let Some(path) = summary.path_components() {
        // Get the last component as the type name
        path.split("::").last().map(str::to_string)
    } else {
        None
    }
}

/// Create a synthetic Item for a workspace external type
/// All workspace types are created as unit structs to ensure they generate containers
fn create_synthetic_item(type_name: &str, summary: &SummaryNode) -> Item {
    let synthetic_id = Id(summary.id.id);
    let inner = ItemEnum::Struct(Struct {
        kind: StructKind::Unit, // Unit struct always generates a container
        generics: Generics {
            params: vec![],
            where_predicates: vec![],
        },
        impls: vec![],
    });

    Item {
        id: synthetic_id,
        crate_id: summary.summary.crate_id,
        name: Some(type_name.to_string()),
        span: None,
        visibility: Visibility::Public,
        docs: None,
        links: HashMap::new(),
        attrs: vec![],
        deprecation: None,
        inner,
    }
}

#[cfg(test)]
#[path = "filter_tests.rs"]
mod filter_tests;
