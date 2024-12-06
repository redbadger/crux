use ascent::ascent;

use super::node::{CrateNode, ItemNode, SummaryNode};

ascent! {
    pub struct Filter;

    // ------- facts ------------------
    relation item(ItemNode);
    relation summary(SummaryNode);
    relation ext_crate(CrateNode);

    // ------- rules ------------------

    relation has_summary(ItemNode, SummaryNode);
    has_summary(n, s) <-- item(n), summary(s), if n.has_summary(s);

    relation is_struct(ItemNode);
    is_struct(s) <-- item(s), if s.is_struct();

    relation is_enum(ItemNode);
    is_enum(e) <-- item(e), if e.is_enum();

    relation variant(ItemNode, ItemNode);
    variant(e, v) <-- is_enum(e), item(v), if e.has_variant(v);

    relation field(ItemNode, ItemNode);
    field(s, f) <-- is_struct(s), item(f), if s.has_field(f);
    field(v, f) <-- variant(e, v), item(f), if v.has_field(f);

    relation type_of(ItemNode, ItemNode);
    type_of(f, t) <-- item(f), item(t), if f.is_of_type(t);

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
        type_of(field, child);

    relation root_app(ItemNode, ItemNode);
    root_app(impl_, app) <--
        app(impl_, app),
        !parent(_, app);

    // a view model is an associated type of an app
    relation view_model(ItemNode, ItemNode);
    view_model(app, view_model) <--
        root_app(impl_, app),
        type_of(item, view_model),
        if impl_.has_associated_item(item, "ViewModel");

    // an event is an associated type of an app
    relation event(ItemNode, ItemNode);
    event(app, event) <--
        root_app(impl_, app),
        type_of(item, event),
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
        type_of(effect_ffi_item, effect_ffi),
        if effect_impl.has_associated_item(effect_ffi_item, "Ffi");

    relation root(ItemNode);
    root(x) <-- view_model(app, x);
    root(x) <-- event(app, x);
    root(x) <-- effect(app, x);

    // set of all the edges we are interested in
    relation edge(ItemNode, ItemNode);

    // root fields
    edge(root, field) <--
        root(root),
        field(root, field);
    // root variants
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
        type_of(field, type_);
}

#[cfg(test)]
#[path = "filter_tests.rs"]
mod filter_tests;
