# Crux CLI codegen for foreign types

The codegen command on the `crux` CLI generates code for foreign types in Swift,
Kotlin and TypeScript.

<!-- prettier-ignore -->
> [!NOTE]
> This is a work in progress and is not yet ready for general use.

```sh
crux codegen --lib shared
```

The `--lib` flag specifies the library to generate code for. The `shared`
library is used in this example.

## How it works

### Source data

Generate `rustdoc` JSON for the library, using the
[`rustdoc_json`][rustdocJsonReference] crate. We want to use the latest format
version so we currently require Rust nightly to be installed
(`rustup install nightly`). This JSON describes all the public and private items
in the library and is deserialized using the
[`rustdoc-types`][rustdocTypesReference] crate.

We currently only look into the specified library and not its dependencies, but
this will change.

### Build a graph

Parse the JSON and build a graph of the relationships that we are interested in.

Start by adding edges for any `impl` blocks that implement the `App` or `Effect`
traits.

Then add edges for the `Event`, `ViewModel`, and `Capabilities` associated types
in the `App` impl.

Then add edges for struct fields and enum variants.

```mermaid
graph TD
    Ty([Type])
    S([Struct])
    SF([Struct Field])
    E([Enum])
    V([Variant])
    I([Impl])
    AI([Associated Item])
    TA([App Trait])
    TE([Effect Trait])
    S --> |Field| SF
    E --> |Variant| V
    V --> |Field| SF
    SF --> |Type| Ty
    I --> |AssociatedItem| AI
    AI --> |AssociatedType| Ty
    I -.-> |TraitApp| TA
    I -.-> |TraitEffect| TE


```

### Process the graph

Process the data using the [`ascent`][ascentCrateReference] crate to run a logic
program (similar to Datalog) on the graph.

```rust
ascent! {
    relation edge(Node, Node, Edge);

    relation app(Node);
    relation effect(Node);
    relation is_effect_of_app(Node, Node);
    relation root(Node);
    relation parent(Node, Node);
    relation field(Node, Node);
    relation variant(Node, Node);

    // app structs have an implementation of the App trait
    app(app) <--
        edge(app_impl, app_trait, Edge::TraitApp),
        edge(app_impl, app, Edge::Type);

    // effect enums have an implementation of the Effect trait
    effect(effect) <--
        edge(effect_impl, effect_trait, Edge::TraitEffect),
        edge(effect_impl, effect, Edge::Type);

    // an effect belongs to an app if they are in the same module
    is_effect_of_app(app, effect) <--
        app(app),
        effect(effect),
        if are_in_same_module(app, effect);

    // Event and ViewModel types are associated
    // with the root apps (that have no parent)
    root(assoc_type) <--
        edge(app_impl, app_trait, Edge::TraitApp),
        edge(app_impl, app, Edge::Type),
        !parent(_, app),
        edge(app_impl, assoc_item, Edge::AssociatedItem),
        edge(assoc_item, assoc_type, Edge::AssociatedType);
    // Effects belong to the root apps (that have no parent)
    root(effect_enum) <--
        is_effect_of_app(app, effect_enum),
        !parent(_, app);

    // app hierarchy
    parent(parent, child) <--
        app(parent),
        app(child),
        edge(parent, field, Edge::Field),
        edge(field, child, Edge::Type);

    // fields of root structs
    field(struct_, field) <--
        root(struct_),
        edge(struct_, field, ?Edge::Variant|Edge::Field);
    // recursive descent
    field(struct2, field2) <--
        field(struct1, field1),
        edge(field1, struct2, Edge::Type),
        edge(struct2, field2, ?Edge::Variant|Edge::Field);

    // variants of root enums
    variant(enum_, variant) <--
        root(enum_),
        edge(enum_, variant, Edge::Variant);
    // recursive descent
    variant(variant, field) <--
        variant(enum_, variant),
        edge(variant, field, Edge::Field);
}
```

### Create an intermediate representation

For now we are using the same IR as the
[`serde_generate`][serdeGenerateReference] crate that we currently use for
typegen. This gives us a backend for free and should allow us to maintain
backwards compatibility.

### Generate foreign types

The IR is used to generate code for foreign types in Swift, Kotlin and
TypeScript, via a vendored version of the
[`serde_generate`][serdeGenerateReference] crate.

We will likely want to change this in the future to allow us to generate more
idiomatic code for each language, and support Crux more fully.

[ascentCrateReference]: https://crates.io/crates/ascent
[rustdocJsonReference]: https://crates.io/crates/rustdoc-json
[rustdocTypesReference]: https://crates.io/crates/rustdoc-types
[serdeGenerateReference]: https://crates.io/crates/serde-generate
