# RFC: App-declared effect groups

```admonish
This RFC is **proposed**. It builds on the
[capability contracts RFC](./documented-effect-handlers.md), which names
effect groups as future work.
```

This RFC proposes that an app can put the variants of its `Effect` enum into
named groups, with `#[codegen(group = "Store")]`. Type generation then emits
one handler interface for each group, and a composite handler that builds an
`EffectHandler` from one handler for each group. A test or a preview can then
replace one group with a fake and keep the real handlers for the others.

## Why?

The generated `EffectHandler` has one method for each effect variant. A shell
implements all of them in one type. This is correct for the app, but it is not
convenient for a test or a preview:

- A SwiftUI preview needs a `Core`, so it needs an `EffectHandler`. Today the
  preview of the weather example implements every method with `fatalError`,
  because it cannot use only some of the real handlers.
- A shell test that fakes the network must also implement the key-value store,
  the timers and the location, or it must subclass the real handler.

The earlier revisions of the capability contracts RFC tried to solve this with
an interface for each capability. That RFC records why each of those designs
was not chosen. The main reason is that a capability crate cannot know which of
its operations an app uses, and a custom capability lives in the app crate. The
app is the only place that knows the groups that are useful to its shells.

## Design

The app puts a variant into a group with an attribute on the variant:

```rust
#[effect(facet_typegen)]
pub enum Effect {
    Render(RenderOperation),
    #[codegen(group = "Network")]
    Http(HttpRequest),
    #[codegen(group = "Store")]
    KvGet(GetValue),
    #[codegen(group = "Store")]
    KvSet(SetValue),
    GetLocation(GetLocation),
}
```

The rules for the attribute are:

- The group name is a string in upper camel case, because it becomes part of a
  type name in four languages. The `#[effect]` macro rejects other names, and
  unknown keys inside `codegen(...)`, at compile time.
- A variant can be in one group only. Variants with the same name are in the
  same group.
- The attribute changes only the generated shell code. The `#[effect]` macro
  removes it from the enums that it emits, so the Rust core does not change.

Type generation emits these types:

- `<Group>Handler` for each group, with the methods of its variants. Each
  method has the name, the signature and the doc of the `EffectHandler` method
  of today (`kvGet`, not `get`).
- `UngroupedHandler` for the variants in no group, except `Render`, which the
  generated `Core` handles.
- `EffectHandler`, which now refines (Swift), extends (Kotlin, TypeScript) or
  inherits (C#) the group interfaces, and declares only `render`.
- `GroupedEffectHandler`, an `EffectHandler` made from one handler for each
  group and one for `ungrouped`. Its parameters are in the order in which each
  group first appears in the `Effect` enum, and `ungrouped` is last.

C# names each interface with an `I` prefix (`IStoreHandler`), as it does for
`IEffectHandler`.

In Swift, the weather preview becomes:

```swift
GroupedEffectHandler(
    network: OfflineNetwork(),
    store: InMemoryStore(),
    time: TaskSleepTimeHandler(),
    ungrouped: FixedLocation()
)
```

Each language forwards the methods in the way that suits it. Kotlin uses
delegation (`StoreHandler by store`). This works because `EffectHandler`
extends the group interfaces and does not declare their methods again. Swift,
TypeScript and C# emit one forwarding method for each variant.

Because `EffectHandler` still has every method through the group interfaces,
one type that implements all of `EffectHandler` still compiles. An app can add
groups and change its shells only when it wants a fake. The dispatcher does
not change.

If no variant has a group, the generated code is the same as today.

Type generation rejects a group if its `<Group>Handler` or `I<Group>Handler`
is the name of a registered type or a name that the shell API uses. Thus
`group = "Effect"` and `group = "Ungrouped"` fail. It also rejects `Render` in
a group, because `EffectHandler` declares `render` itself.

## Drawbacks

**The methods move out of `EffectHandler`.** A reader who opens
`EffectHandler` sees only `render` and a list of group interfaces. The methods
are in the group interfaces.

**The order of the parameters follows the enum.** If the app moves a variant so
that a group appears earlier, the call to `GroupedEffectHandler` in each shell
must change. The arguments have different interface types, so the compiler
finds a wrong order, unless one object implements both interfaces. Swift
labels also make the order visible at the call.

**Each language needs generator code.** The spike measured about 175 lines of
shared code, 150 for Swift and 110 for Kotlin. TypeScript and C# need about
140 each.

## Alternatives considered

**`#[effect(group = "Store")]`.** The spike used the name of the macro. The
attribute changes only generated shell code and never the Rust core, so
`codegen` says better what it does. It also leaves `effect` free for options
that change the Rust code.

**Groups by crate.** Type generation could group the variants by the crate
that defines the operation. A custom capability lives in the app crate,
together with the app's other operations, so the crate is not a useful group.
The capability contracts RFC records the other designs with one interface for
each capability.

**A group key that is an identifier, not a string.** `group = Store` looks like
a path to a type, and readers and rust-analyzer would look for a `Store` type.
A string is the usual form for a name in an attribute, as in
`#[serde(rename = "...")]`.

**A composite with a parameter object, or in alphabetical order.** A parameter
object (`{ store, network }`) is not sensitive to order, but it adds a public
type in each language. Alphabetical order does not change when the app moves a
variant, but it is less obvious to the reader of the enum.

**No composite.** Each shell could write its own forwarding type. This is about
one line for each variant in Swift, TypeScript and C#, and every app would
write the same code.

## Unresolved questions

- Is `GroupedEffectHandler` the correct name for the composite? `AppHandler`
  and `EffectHandlers` are other options.
- Is `UngroupedHandler`, with the parameter `ungrouped`, clear enough for the
  variants in no group?
