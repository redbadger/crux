# RFC: Capability contracts in the generated `EffectHandler`

```admonish
This RFC is **proposed**. It builds on the
[per-operation types RFC](./per-operation-types.md) and the
[generated `Core` RFC](./generated-core.md). Nothing in it depends on the
shape of the `Operation` trait, which the
[operation kind traits RFC](./operation-kind-traits.md) proposes to change in
the breaking release.
```

This RFC proposes that a capability states its contract with the shell in the
Rust doc comments of its operation and output types, and that type generation
writes the doc comment of each operation on its method in the generated
`EffectHandler`, in Swift, Kotlin, TypeScript and C#. Capability crates would
ship no shell source, and an app would register nothing.

## Why?

After the per-operation types RFC, a shell implements the generated
`EffectHandler`: one method for each effect variant, with a typed operation in
and a typed output out. The types say what goes over the bridge. They do not
say what the values mean. For example:

- An `HttpRequest` answers `Ok` for every response that arrives, whatever its
  status. `Err` means that no response arrived.
- A `GetValue` for a key with no value answers `Ok` with `Value::None`, not an
  error. A `SetValue` answers with the value that it replaced.
- A `NotifyAfter` can still answer after a `ClearTimer` for its id, and the
  core ignores that answer.

Today a shell author finds these facts in the book and in the example shells.
The capability author knows them, and the Rust doc of the operation is the
place that the author keeps up to date when the capability changes. If the
generated handler method carries that doc, the shell author reads the contract
where they write the code, in their own language.

## Design

`EffectBuilder::variant`, which the `#[effect]` macro calls for each variant,
records the doc comment of the operation type from its `Shape`. Thus each
operation must implement `Facet`, which every operation that an effect carries
already does. The handler plugin writes the doc on the method with the
`write_doc` function of facet_generate for each language. For example, in
Swift:

```swift
public protocol EffectHandler: Sendable {
    /// Read the bytes stored under `key`.
    ///
    /// The shell answers with a `ValueResult`. If `key` has no value, the answer
    /// is `Ok` with `Value::None`, not an error.
    func kvGet(_ operation: GetValue) async -> ValueResult
    // ...
}
```

The doc describes the wire format: the values that the shell sends back and
what each one means. How a platform client maps its own errors onto those
values (for example, which `URLError` is a timeout) stays with the shell.

The operations of `crux_http`, `crux_kv` and `crux_time` get the contract text
in their docs. The text is written once, in Rust, and is neutral to the
language of the shell. Rustdoc link syntax (`` [`ClearTimer`] ``) passes
through to the generated doc unchanged. A change in facet_generate will strip
it.

An operation of the app itself gets the same treatment. Its doc comment
appears on its handler method, with no extra work.

## Drawbacks

**No compiler check of the contract.** A doc comment states the contract, but
nothing makes a shell follow it. The alternatives below that add an interface
for each capability do not check it either: an interface checks the method
signatures, which `EffectHandler` already checks.

**The doc is shared by the Rust and shell readers.** A sentence that only
helps a Rust caller (for example, a link to the Rust builder) also appears in
every shell. Capability authors must write operation docs for both readers.

## Alternatives considered

The earlier revisions of this RFC proposed that each capability crate ship the
shell side of its contract as source. The review found problems with each way
to do that.

**Implementations in the capability crates.** Each capability crate ships a `URLSession`,
OkHttp, `fetch` or `HttpClient` implementation. An implementation belongs to
each app and shell: each app configures its client and chooses where its data
goes. One implementation that suits every app is hard to make, and four of
them are a cost to every capability author.

**Explicit registration of a protocol.** The app registers each capability's
protocol in its codegen binary. This adds a step that the app can forget, and
the protocol must still list every operation of the capability, including the
ones the app does not use.

**Discovery through a `protocol =` back-link on each operation.** This was the
previous design. Each operation names its protocol, and the derive registers
it. It needs a back-link on every operation and a hidden macro of `crux_core`
in each capability crate, and every app gets every type of the capability.

**Discovery through a facet attribute.** A facet extension attribute on the
operation carries the protocol source (`shape_type` and `include_str!`
payloads work). It removes the hidden macro, but the back-link stays.

**Generated protocols grouped by crate.** Type generation groups the
operations by the crate that defines them. A custom capability lives in the
app crate, together with the app's other operations, so the crate is not a
useful group.

**Generated protocols with a capability label or marker type.** Each operation
names a `capability =` label or a marker type, and type generation emits one
protocol for each capability. The protocol holds only the operations that the
app uses, so it changes from app to app, and it is only a regrouping of
`EffectHandler`.

**A forward list of operations on a marker type.** The marker lists its
operations, so that the protocol is complete. Type generation cannot find the
marker without an explicit registration or a global registry.

**Generated forwarding to capability protocols.** Type generation also emits
the forwarding from `EffectHandler` to each protocol. A spike (commit
`lpqrrkro` in `crux-spike-e`) built it for Swift. The weather shell got 7
lines longer than on master. The estimate for four languages is about 1,000
lines of generator. The back-link stayed.

## Future work

**App-declared effect groups.** The app groups its own effect variants, for
example with `#[effect(group = "Store")]`. Type generation emits a handler
protocol for each group, and `EffectHandler` refines them, so that a test or a
preview can replace one group with a fake. Capability crates do not change,
and an app with no groups gets the same output as today. A spike (commit
`ukqqrxvn` in `crux-spike-e`) built it for Swift and Kotlin. It would get its
own RFC.
