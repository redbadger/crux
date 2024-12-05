use ascent::ascent;

use super::node::Node;

ascent! {
    pub struct Filter;

    // ------- facts ------------------
    relation node(Node);

    // ------- rules ------------------

    relation is_struct(Node);
    is_struct(s) <-- node(s), if s.is_struct();

    relation is_enum(Node);
    is_enum(e) <-- node(e), if e.is_enum();

    relation variant(Node, Node);
    variant(e, v) <-- is_enum(e), node(v), if e.has_variant(v);

    relation field(Node, Node);
    field(s, f) <-- is_struct(s), node(f), if s.has_field(f);
    field(v, f) <-- variant(e, v), node(f), if v.has_field(f);

    relation type_of(Node, Node);
    type_of(f, t) <-- node(f), node(t), if f.is_of_type(t);

    // app structs have an implementation of the App trait
    relation app(Node, Node);
    app(imp, app) <--
        is_struct(app),
        node(imp),
        if imp.is_impl_for(app, "App");

    // app hierarchy
    relation parent(Node, Node);
    parent(parent, child) <--
        app(_, parent),
        app(_, child),
        field(parent, field),
        type_of(field, child);

    relation root_app(Node, Node);
    root_app(impl_, app) <--
        app(impl_, app),
        !parent(_, app);

    // a view model is an associated type of an app
    relation view_model(Node, Node);
    view_model(app, view_model) <--
        root_app(impl_, app),
        type_of(item, view_model),
        if impl_.has_associated_item(item, "ViewModel");

    // an event is an associated type of an app
    relation event(Node, Node);
    event(app, event) <--
        root_app(impl_, app),
        type_of(item, event),
        if impl_.has_associated_item(item, "Event");

    // effect enums have an implementation of the Effect trait
    // and an associated Ffi type, which is the FFI representation of the effect
    relation effect(Node, Node);
    effect(app, effect_ffi) <--
        root_app(app_impl, app),
        is_enum(effect),
        node(effect_impl),
        if effect_impl.is_impl_for(effect, "Effect"),
        if app.is_in_same_module_as(effect),
        type_of(effect_ffi_item, effect_ffi),
        if effect_impl.has_associated_item(effect_ffi_item, "Ffi");

    relation root(Node);
    root(x) <-- view_model(app, x);
    root(x) <-- event(app, x);
    root(x) <-- effect(app, x);

    // set of all the edges we are interested in
    relation edge(Node, Node);

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
