#![allow(clippy::no_effect_underscore_binding)]

use ascent::ascent;
use log::{debug, info};
use rustdoc_types::Crate;

use super::item::{is_enum, is_relevant, is_struct, is_struct_unit, is_use};
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

    // Re-exported types from dependency crates should be treated as root types
    // This finds types that are re-exported via "pub use" statements
    relation reexported_type(ItemNode);
    reexported_type(type_) <--
        item(use_item),
        if is_use(&use_item.item),
        if use_item.is_public_reexport(),
        if use_item.id.crate_ == "shared",  // Only process re-exports from the main crate
        item(type_),
        if use_item.reexports_type(&type_),
        if type_.id.crate_ != "shared",  // Only include types from dependency crates
        if {
            log::debug!("Found re-exported type: {} from crate {} via use item in shared", 
                type_.name().unwrap_or("unknown"), 
                type_.id.crate_);
            true
        };

    // Types from dependency crates that are used as fields in root types should also be root types
    // This needs to be recursive to handle nested structures
    relation public_field_type(ItemNode);
    
    // Direct field types from primary roots
    public_field_type(type_) <--
        primary_root(root_type),
        field(root_type, field),
        local_type_of(field, type_),
        if type_.id.crate_ != "shared",
        if {
            log::debug!("Found public field type: {} from crate {} (used in {})", 
                type_.name().unwrap_or("unknown"), 
                type_.id.crate_,
                root_type.name().unwrap_or("unknown"));
            true
        };
    public_field_type(type_) <--
        primary_root(root_type),
        field(root_type, field),
        remote_type_of(field, summary),
        item(type_),
        has_summary(type_, summary),
        if type_.id.crate_ != "shared",
        if {
            log::debug!("Found public field type (remote): {} from crate {} (used in {})", 
                type_.name().unwrap_or("unknown"), 
                type_.id.crate_,
                root_type.name().unwrap_or("unknown"));
            true
        };
    
    // Also check fields of structs that are used as field types (one level deep)
    public_field_type(type_) <--
        primary_root(root_type),
        field(root_type, field),
        local_type_of(field, intermediate_type),
        if intermediate_type.id.crate_ == "shared",
        field(intermediate_type, nested_field),
        local_type_of(nested_field, type_),
        if type_.id.crate_ != "shared",
        if {
            log::debug!("Found nested public field type: {} from crate {} (used in {} -> {})", 
                type_.name().unwrap_or("unknown"), 
                type_.id.crate_,
                root_type.name().unwrap_or("unknown"),
                intermediate_type.name().unwrap_or("unknown"));
            true
        };

    // First define primary roots (without public_field_type to avoid circular dependency)
    relation primary_root(ItemNode);
    primary_root(x) <-- view_model(app, x);
    primary_root(x) <-- event(app, x);
    primary_root(x) <-- effect(app, x);
    primary_root(x) <-- operation(op_impl, x);
    primary_root(x) <-- output(x);
    primary_root(x) <-- reexported_type(x);

    relation root(ItemNode);
    root(x) <-- primary_root(x);
    root(x) <-- public_field_type(x);

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
    
    // Also follow remote type relationships to include types from dependency crates
    edge(field, type_) <--
        edge(_, field),
        remote_type_of(field, summary),
        item(type_),
        has_summary(type_, summary);

    // For remote types that are included, ensure they get processed like root types
    // This means including their struct/enum relationships and field edges
    edge(type_, field) <--
        edge(_, type_),
        is_struct(type_),
        item(field),
        if type_.has_field(field);

    edge(type_, variant) <--
        edge(_, type_),
        is_enum(type_),
        item(variant),
        if type_.has_variant(variant);

    // Follow field types from these newly included struct fields
    edge(field, field_type) <--
        edge(type_, field),
        local_type_of(field, field_type);

    edge(field, field_type) <--
        edge(type_, field),
        remote_type_of(field, summary),
        item(field_type),
        has_summary(field_type, summary);

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
        
        // Count use items for debugging
        let use_count = crate_.index.values().filter(|item| is_use(item)).count();
        if use_count > 0 && crate_name == "shared" {
            use rustdoc_types::{Item, ItemEnum};
            
            debug!("Found {use_count} use items in crate {crate_name}");
            
            // Log some examples of use items
            for item in crate_.index.values() {
                if let Item { inner: ItemEnum::Use(use_item), visibility, .. } = item {
                    if matches!(visibility, rustdoc_types::Visibility::Public) {
                        if use_item.source.starts_with("models::") {
                            debug!("  Public re-export from models: {} -> {}", use_item.source, use_item.name);
                        } else if use_item.source.starts_with("progress::") {
                            debug!("  Public re-export from progress: {} -> {}", use_item.source, use_item.name);
                        }
                    }
                }
            }
        }
        
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
}

#[cfg(test)]
#[path = "filter_tests.rs"]
mod filter_tests;
