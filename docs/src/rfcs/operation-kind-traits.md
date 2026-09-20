# RFC: One trait per operation kind

```admonish
This RFC is **proposed**. It builds on the
[per-operation types RFC](./per-operation-types.md) and supersedes that RFC's
breaking stage — the target shape its Design section originally specified,
reproduced below under [What the parent proposed](#what-the-parent-proposed)
— and it is not yet implemented. The shape it proposes, and each of the
alternatives it rejects, have been compiled and checked on stable Rust, but
nothing here has shipped; if accepted, it would land in the next major
release of `crux_core`.
```

This RFC proposes that the three operation kinds — notify, request and stream
— each become a trait that names the payload an operation of that kind is
answered with, under the word that fits the kind: a request's `Response`, a
stream's `Item`, and nothing for a notification. `Operation` survives as the
supertrait of all three, keeping the kind as an associated type and its
existing `Output`, which the machinery generic over every kind reads. What
changes about `Operation` is who writes it: only `#[derive(Operation)]` does.
The trait is hidden from the documentation and sealed, so an author declares
an operation with one attribute and never sees two impls, and nothing an
author can write disagrees with itself.

Review asked whether the supertrait is needed at all. This RFC answers that it
is, for a reason narrower than the one it first gave: the `#[effect]` macro
sees each variant as a type path and nothing else, and everything it generates
for that variant has to work whatever the operation's kind is. Without one
trait every kind implements, the macro cannot be written. The
[Alternatives](#alternatives-considered) section shows the three ways of
removing the trait that Rust rejects, and the one that works, which costs
the effect enum having to name each variant's kind.

## Summary

The compat release of per-operation types declares an operation's kind and
its payload in two or three places. A hand-written operation writes
`type Output` and `const KIND` on `Operation`, then implements one of the
markers `operation::{Notify, Request, Stream}`; the derive writes all of it,
so it cannot disagree with itself, but the trait shape lets a hand-written
impl declare `KIND = Some(OperationKind::Notify)` and implement
`operation::Request`, and nothing checks one against the other. And `Output`
means three different things: the single answer to a request, each item of a
stream, and, for a notification, a `()` that has to be there because the trait
demands a type.

The proposal is to keep `Operation` and its `Output`, make the kind an
associated type, hide the trait, and have each kind trait name the payload
under its own word, bound to `Output` so that the two are one declaration:

```rust
// crux_core::capability — #[doc(hidden)]; only the derive implements it
pub trait Operation: __private::Sealed + Send + 'static {
    type Kind: operation::Kind;
    type Output: Send + Unpin + 'static; // unchanged from today
}

// crux_core::operation — public; what bounds are written against
pub mod operation {
    pub trait Notify: Operation<Kind = kind::Notify, Output = ()> {}

    pub trait Request:
        Operation<Kind = kind::Request, Output = <Self as Request>::Response>
    {
        type Response: Send + Unpin + 'static;
    }

    pub trait Stream:
        Operation<Kind = kind::Stream, Output = <Self as Stream>::Item>
    {
        type Item: Send + Unpin + 'static;
    }
}
```

An author writes the derive and one attribute:

```rust
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
#[operation(request, response = ValueResult)]
pub struct Get { pub key: String }
```

and the derive writes the three impls that attribute stands for — the sealing
marker, `Operation` naming `Kind` and `Output`, and the kind trait naming the
same payload under the kind's own word:

```rust
impl crux_core::__private::Sealed for Get {}

impl Operation for Get {
    type Kind = operation::kind::Request;
    type Output = ValueResult;
}

impl operation::Request for Get {
    type Response = ValueResult;
}
```

Four things follow. The wrong `Command` constructor is a trait-bound error
that `cargo check` and the editor report, with an authored message naming the
right constructor, rather than the compat release's post-monomorphisation
`const` assertion. The payload has the right name where an author writes it
and where kind-specific code reads it. `Request<Op>`, the bridge registry, the
effect router, middleware and type generation keep reading `Op::Output` from a
single `Op: Operation` bound, exactly as they do today, so none of them
changes. And because nothing outside `crux_core` implements `Operation`, its
items can change in a minor release: a future binding for shipped shell
handlers, or a change to how the kind is carried, would not be breaking.

## Why?

### The kind and the payload are declared in more than one place

Under the compat release, this is a request written by hand:

```rust
impl Operation for Get {
    type Output = ValueResult;
    const KIND: Option<OperationKind> = Some(OperationKind::Request);
}

impl crux_core::operation::Request for Get {}
```

The kind appears twice, as a `const` and as a marker impl. The `Command`
constructors check the `const`, in a `const { assert!(..) }` block that only
fails once the offending call is monomorphised; their bounds are still
`Op: Operation`, so the markers are not what the constructors look at. The
markers exist so that bounds elsewhere can name a kind, and only `Notify`
constrains anything — `Operation<Output = ()>`. So `KIND = Some(Notify)` next
to `impl operation::Request` compiles, and the two halves of the declaration
disagree about what the operation is. The derive never produces that, and no
code in the repository does, but the trait shape permits it. Reviewing the
derive ([#583](https://github.com/redbadger/crux/pull/583)), @charypar put it
this way:

> It looks quite possible for someone to `impl Operation` wrongly on a type,
> e.g. implement both `Notify` and `Operation` with `KIND: StreamKind`, and I
> don't love that only the macro really prevents that. I think we can probably
> make that inconsistency impossible when we remove `Operation` later, unless
> we absolutely need it.

The parent RFC's answer was to make the kind an associated type,
`type Kind: Kind`, with sealed unit types `kind::{Notify, Request, Stream}`,
and to make the markers blanket impls from it, so that the kind is declared
once and the marker follows. That much is kept here. What it left in place is
the payload, and the possibility of a hand-written impl at all.

### `Output` means three things

`Operation::Output` is the type a request is answered with, the type of each
item a stream yields, and `()` for a notification, which is answered with
nothing. One name for three meanings has three costs.

For a stream, `Output` is the wrong word — the parent RFC says so, and
concedes it: `type Item` "would need a second associated type on `Operation`
or a way to express 'Output is the item' that generic code can use
uniformly". The derive's argument for a stream is spelled `output =` for the
same reason: while the three kinds share one trait, a stream cannot have a
name of its own. Both were raised in review of the derive (#583) and deferred
to the breaking release.

For a notification, `Output = ()` is a value that will never exist. Review of
the RFC ([#581](https://github.com/redbadger/crux/pull/581)) and of the derive
(#583) asked for `Output = !`, because a request can in principle resolve with
a unit while a notification must never resolve at all; the never type is not
stable in that position on stable Rust (E0658), so the compat release pins
`()` and adds a marker bound and a compile-fail test to hold it there.

And for a request, `Output` is an adequate word but not the domain's. The
bridge's own errors speak of a "response" — "Request 1 expects `Http`, but
response id 0x01000001 carries `Render`" — and the compat release's
`KeyValueResponse` and `TimeResponse` were named for what they are. Review of
the migration guide ([#588](https://github.com/redbadger/crux/pull/588)) asked
that the guide say the compat shape is temporary and that the request's
payload type will move to the `Request` trait. This RFC is the proposal that
comment asked for.

### The derive already writes every operation that declares a kind

Of the operations in this repository that declare a kind, two are written by
hand: `crux_http`'s `HttpRequest` and `crux_core`'s own `RenderOperation`.
Every other one — `crux_kv`'s five, `crux_time`'s four, the `notes` example's
notification and stream, the `weather` example's five — is a derive and an
attribute. The derive handles generic operations and `where` clauses, and a
type from another crate cannot implement a `crux_core` trait by hand either,
because of the orphan rules; it has to be wrapped, and the wrapper can derive.
So a hand-written `impl Operation` buys nothing the derive does not, and it is
the only way to write the inconsistency review objected to. This RFC takes it
away.

## Goals

1. The name an author writes for an operation's payload is the right one for
   its kind: `Response` for a request, `Item` for a stream, nothing for a
   notification.
2. An operation's kind and payload cannot be declared inconsistently, because
   only the derive declares them. A hand-written `impl Operation` is not
   supported.
3. The wrong `Command` constructor is a trait-bound error, visible to
   `cargo check` and the editor, that names the right constructor.
4. The machinery generic over every kind — `Request<Op>`, the bridge registry,
   the effect router, middleware, type generation, the `#[effect]` macro's
   generated helpers — keeps a single `Op: Operation` bound and keeps reading
   `Op::Output`, so it does not change.
5. Code written against the compat release's bounds and derive changes as
   little as possible, and every change is a rename.
6. `Operation` can change in a minor release, because nothing outside
   `crux_core` implements it.

## Non-goals

- Changing the wire format, the generated shell API, or anything a shell
  sees. The kind reaches shells the way it does today.
- Changing how an effect enum is written. `#[effect]` keeps taking
  `Http(HttpRequest)`; the variant does not say what kind `HttpRequest` is.
  This is the constraint the rest of the design answers to.
- Redesigning `Command`, the effect router or middleware. The `Command` and
  `CommandContext` bounds tighten to the kind traits, as the parent RFC
  proposed. Nothing else changes.
- Using the never type. `operation::Notify` declaring no payload type of its
  own is what makes `Output = !` unnecessary; this RFC does not propose
  adopting `!` when it becomes available.
- Removing `Operation`. The Design section explains why it cannot go, and the
  Alternatives section shows what removing it would cost.

## Design

### What the parent proposed

The per-operation types RFC's Design section specified this target shape for
the breaking release. It is reproduced here as it was written, because
reviewers approved it and this RFC replaces it; that RFC now defers to this
one.

```rust
/// How many times a request expects to be resolved. Introduced by PR #580.
pub enum OperationKind { Notify, Request, Stream }

pub mod operation {
    /// Common base: a serializable payload the shell can act on. Carries the
    /// typegen registration hooks that `Operation` carries today.
    pub trait Operation: Send + 'static {
        /// The value the shell resolves with. `()` for notifications.
        type Output: Send + Unpin + 'static;

        /// How many times this operation expects to be resolved.
        type Kind: Kind;

        // register_types / register_types_facet as today
    }

    /// One of the three kinds, as a type. Sealed; the only impls are below.
    pub trait Kind: sealed::Sealed {
        const VALUE: OperationKind;
    }

    pub mod kind {
        pub struct Notify;   // impl Kind { VALUE = OperationKind::Notify }
        pub struct Request;  // impl Kind { VALUE = OperationKind::Request }
        pub struct Stream;   // impl Kind { VALUE = OperationKind::Stream }
    }

    /// Fire and forget. Nothing waits on it.
    pub trait Notify: Operation<Output = (), Kind = kind::Notify> {}
    impl<Op: Operation<Output = (), Kind = kind::Notify>> Notify for Op {}

    /// Exactly one response.
    pub trait Request: Operation<Kind = kind::Request> {}
    impl<Op: Operation<Kind = kind::Request>> Request for Op {}

    /// Zero or more responses.
    pub trait Stream: Operation<Kind = kind::Stream> {}
    impl<Op: Operation<Kind = kind::Stream>> Stream for Op {}
}
```

> `Operation` keeps `Output` and gains `Kind`, so `Request<Op>`, the bridge
> registry, the effect router and middleware keep working against one trait
> and can read the kind statically as `<Op::Kind as Kind>::VALUE`. The three
> marker traits, `operation::{Notify, Request, Stream}`, exist so that bounds
> can name a kind. They are blanket-implemented from `Kind`, so an author
> declares the kind exactly once and nothing can disagree with it.
>
> `operation::Stream` uses `Output` for the item type. `type Item` would read
> better, but it would need a second associated type on `Operation` or a way
> to express "Output is the item" that generic code can use uniformly.

Everything above the kind traits is kept: `OperationKind`, `type Kind` on
`Operation`, the sealed `Kind` trait and its three unit types, and `Output`
staying on `Operation`. Two things change. The three kind traits stop being
payload-free markers and carry the payload type under the kind's own word.
And `Operation` stops being something an author implements: the derive writes
it, and it is hidden and sealed so that nothing else does. The concession
about `type Item` dissolves, because the uniform name generic code needs is
`Output` on the supertrait, and the kind trait is free to use the word that
reads best.

### The shape

```rust
pub mod capability {
    /// Implemented by `#[derive(Operation)]`, never by hand. Generic code
    /// reads it; an author reads the kind traits.
    #[doc(hidden)]
    pub trait Operation: crate::__private::Sealed + Send + 'static {
        /// How many times this operation expects to be resolved, as a type.
        type Kind: operation::Kind;

        /// The value the shell resolves with, named without reference to the
        /// kind: `()` for a notification, a request's `Response`, a stream's
        /// `Item`. Generic code reads this one.
        type Output: Send + Unpin + 'static;

        // register_types_facet as today, bounded on Self::Output
    }
}

#[doc(hidden)]
pub mod __private {
    /// The derive implements this alongside `Operation`. The path says what
    /// the documentation cannot: this is not for you to write.
    pub trait Sealed {}
}

pub mod operation {
    pub trait Kind: sealed::Sealed {
        const VALUE: OperationKind;
    }

    pub mod kind {
        pub struct Notify;
        pub struct Request;
        pub struct Stream;
    }

    /// Told to the shell and never answered. There is no payload to name.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is not a notification",
        label = "not a notification",
        note = "a request is sent with request_from_shell, a stream with stream_from_shell"
    )]
    pub trait Notify: Operation<Kind = kind::Notify, Output = ()> {}

    /// Answered exactly once, with a `Response`.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is not a request",
        label = "not a request",
        note = "a notification is sent with notify_shell, a stream with stream_from_shell"
    )]
    pub trait Request:
        Operation<Kind = kind::Request, Output = <Self as Request>::Response>
    {
        type Response: Send + Unpin + 'static;
    }

    /// Answered any number of times, each time with an `Item`.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is not a stream",
        label = "not a stream",
        note = "a notification is sent with notify_shell, a request with request_from_shell"
    )]
    pub trait Stream:
        Operation<Kind = kind::Stream, Output = <Self as Stream>::Item>
    {
        type Item: Send + Unpin + 'static;
    }
}
```

Each kind trait has `Operation` as a supertrait, with its `Kind` pinned to the
matching unit type and its `Output` pinned to the kind trait's own payload
type — `()` for `Notify`, `<Self as Request>::Response` for `Request`,
`<Self as Stream>::Item` for `Stream`. The supertrait bound is what makes the
impls one declaration: implementing `operation::Request` for a type obliges
its `Operation` impl to say `Kind = kind::Request` and
`Output = <that type's Response>`, and the compiler checks the obligation
where the kind trait is implemented. The derive writes both, so the check
never fails; it is there so that the shape is sound on its own terms, not
only because of who writes it.

`Notify` pins `Output = ()`. Nothing reads a notification's payload, so the
type could be left free, but pinning it gives the derive one right answer and
keeps the compat release's guarantee that a notification cannot claim a
payload. It is the price of `Request<Op>` holding notifications in the same
container as everything else, and it is one line the derive writes and no
author reads.

`const KIND` goes, and with it its `Option`: every operation declares a kind,
because an associated type cannot default. `<Op::Kind as Kind>::VALUE` is the
static kind, readable in any generic context that has `Op: Operation`, which
is what `Op::KIND` gave type generation in the compat release.

`Operation` is `#[doc(hidden)]` and has `__private::Sealed` as a supertrait.
The sealing is by convention, in the way `serde` and others do it: the marker
lives in a hidden module whose path names its purpose, the derive implements
it, and a determined author can too. Rust has no way to seal a trait against a
downstream crate while a proc macro in another crate implements it, so this
is as far as the language goes. What it achieves is enough: the documented
way to declare an operation is the derive, an author who writes
`impl crux_core::__private::Sealed for Get {}` knows they are off the path,
and `crux_core` is free to treat `Operation`'s items as its own.

The `#[diagnostic::on_unimplemented]` messages are the static form the
attribute allows. It accepts only `message`, `label` and `note`, and
interpolates only the type itself, `{Self}`; it rejects conditional clauses
and associated-type interpolation, so a message cannot say what kind the type
*is*, only what it is not. "`Get` is not a notification", with a note naming
the two constructors that send the other kinds, is what stable Rust can
produce, and it is quoted from the compiler below.

### Why `Operation` survives

The obvious alternative, once each kind has a trait of its own, is to remove
`Operation` altogether and leave `Notify`, `Request` and `Stream` as the only
traits, with `Notify` having no associated type at all. Review of the derive
(#583) suggested exactly that, and in the same breath saw the difficulty:

> not sure how we're going to write the trait bound for a function which
> takes an effect and generates it's ID depending on the kind if we remove
> `Operation` later. Maybe something like a blanket impl of a trait for each
> of the three subtraits?

An earlier draft of this RFC answered that `Request<Op>` needs one bound from
which to name the payload. That is true but it is not the constraint, because
`Request<Op>` could be replaced. The constraint is the `#[effect]` macro.

An effect enum is written as `Http(HttpRequest)`. The macro sees the variant's
name and a type path, and from those two tokens it generates the field type
`Request<HttpRequest>` with a typed handle inside it, `From` and `TryFrom`
between the container and the enum, the `EffectFFI::serialize` arm that turns
the request into bytes and a deserialising resolver, type generation's
`.variant::<HttpRequest>("Http")` and `HttpRequest::register_types_facet`,
and the `resolve_http` test helper whose closure returns the payload. Every
one of those is a single, kind-agnostic piece of code that has to be correct
whether `HttpRequest` is a notification, a request or a stream, and the macro
has no way to find out which. A kind-agnostic call resolves through a trait
that every kind implements, and there are only so many ways to obtain one.
Stable Rust rejects all but one.

**Blanket impls from the three kind traits** — the shape the review comment
reached for. Three impls of one trait, each bounded on a different kind trait,
overlap: nothing proves that a type cannot implement two of the kind traits,
and coherence does not accept a proof that the kinds are disjoint, because
there is no way to state one.

```text
error[E0119]: conflicting implementations of trait `Operation`
 --> blanket_overlap.rs:9:1
  |
8 | impl<Op: Request> Operation for Op { type Output = Op::Response; }
  | ---------------------------------- first implementation here
9 | impl<Op: Stream> Operation for Op { type Output = Op::Item; }
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ conflicting implementation
```

**A trait on the container, written by the derive.** If the unifying trait
were implemented for `Request<Get>` rather than for `Get`, the three impls
would be on three distinct types and could not overlap — but the derive runs
in the author's crate, and `Request` and the trait are both `crux_core`'s.
That is an orphan impl, and the language forbids it whatever the type
parameter is.

```text
error[E0117]: only traits defined in the current crate can be implemented for types defined outside of the crate
 --> container_orphan.rs:6:1
  |
6 | impl crux_fake::Envelope for crux_fake::Request<Get> { type Output = String; }
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^-----------------------
  |                              |
  |                              `crux_fake::Request` is not defined in the current crate
  |
  = note: impl doesn't have any local type before any uncovered type parameters
```

**Erasing the payload inside the container.** `Request<Op>` could hold a
type-erased handle and recover the payload type where the kind is known, at
`Request::resolve` under an `Op: operation::Request` bound. It fails at the
bridge. The deserialising resolver the bridge stores has to be built where the
payload type is known, which is inside `request_from_shell`; but that
function knows neither the FFI format the bridge will use nor that the payload
is deserialisable at all, and the effect router's opaque lane exists precisely
for payloads that are not. The information the bridge needs is available only
at the macro's kind-agnostic call, which is where the design has nothing to
resolve it with.

What remains is a single trait on the operation type, implemented once per
operation, that names the payload for generic code. That is `Operation`. Its
only alternative is for the effect enum to state each variant's kind, so that
the macro can generate kind-specific code; the Alternatives section works
that design through and gives its cost. Keeping the supertrait costs one
hidden trait and one line per derived notification; removing it costs every
effect enum ever written.

The question was also put the other way round: where does the runtime *need*
to treat the kinds uniformly, and could `Core::resolve` for requests and a
separate `Core::send_item` for streams do without a common trait? At the
resolve site it already does. `Core::resolve<Output>` takes
`&mut impl Resolvable<Output>`, `Request<Op>` implements
`Resolvable<Op::Output>`, and the handle inside it — the `RequestHandle`
enum's `Once` or `Many` — decides whether this is a request being answered
or a stream receiving an item. The indirection is hidden in the handle, as
the review guessed it might be, and no split of `Core::resolve` is needed.
That the runtime manages without a common trait is exactly why it is easy to
believe the trait is unnecessary; the macro is where the belief fails.

### Why only the derive writes it

An earlier draft of this RFC compared two ways of binding the kind traits to
the supertrait and asked review to choose. Sealing the supertrait makes the
choice, by taking the best of each.

With *tied* bounds — the shape above, written by hand — an author writes two
impls, and the compiler rejects a disagreement between them where the kind
trait is implemented. The wrong constructor is an unimplemented trait, so
`#[diagnostic::on_unimplemented]` fires. But a hand-written `Operation` impl
that declares `Kind = kind::Request` and never implements `operation::Request`
compiles, satisfies every generic site, and cannot be sent; the diagnostic
then says it is not a request, which its own impl claims it is.

With *blanket* impls — the parent RFC's shape, the kind traits derived from
`Kind` — an author writes one impl and the disagreement and the orphan cannot
be written at all. But the wrong constructor is then an associated-type
mismatch in the blanket impl's `where` clause, E0271, which the attribute
cannot decorate; the author reads the compiler's walk through the bounds
rather than a sentence.

When the derive is the only author, the tied shape loses its two weaknesses.
Nothing can disagree, because one macro writes both impls from one attribute.
Nothing can be orphaned, for the same reason. And the diagnostic is the
authored one. This is the transcript, from a standalone reproduction of the
proposed shape compiled outside the repository with rustc 1.98.1, edition
2021, abridged to the lines that matter; the file name and line numbers are
the reproduction's, not anything in `crux_core`:

```text
error[E0277]: `Get` is not a notification
   --> derive_only.rs:121:37
    |
121 |     let _ = crux_core::notify_shell(Get); // wrong constructor
    |             ----------------------- ^^^ not a notification
    |             |
    |             required by a bound introduced by this call
    |
help: the trait `operation::Notify` is not implemented for `Get`
   --> derive_only.rs:102:1
    |
102 | pub struct Get;
    | ^^^^^^^^^^^^^^
    = note: a request is sent with request_from_shell, a stream with stream_from_shell
help: the trait `operation::Notify` is implemented for `Publish`
```

The same reproduction, with the offending line removed, compiles and runs:
one operation of each kind, three constructors bounded on the kind traits, a
`Request<Op>` generic over all three, and a function generic over
`Op: Operation` reading `<Op::Kind as Kind>::VALUE`.

### What the derive emits

`#[derive(Operation)]` writes three impls: the sealing marker, `Operation`,
and the kind trait. Its arguments become:

```rust
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
#[operation(notify)]
pub struct Publish(pub Vec<u8>);
// impl __private::Sealed for Publish {}
// impl Operation for Publish { type Kind = kind::Notify; type Output = (); }
// impl operation::Notify for Publish {}

#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
#[operation(request, response = ValueResult)]
pub struct Get { pub key: String }
// impl __private::Sealed for Get {}
// impl Operation for Get { type Kind = kind::Request; type Output = ValueResult; }
// impl operation::Request for Get { type Response = ValueResult; }

#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
#[operation(stream, item = Message)]
pub struct Subscribe;
// impl __private::Sealed for Subscribe {}
// impl Operation for Subscribe { type Kind = kind::Stream; type Output = Message; }
// impl operation::Stream for Subscribe { type Item = Message; }
```

The payload argument takes the kind trait's word: `response =` on a request,
`item =` on a stream, nothing on a notification. `output =` goes. It named a
type called `Output` on a trait the author could see; with `Operation` hidden
there is no `Output` in the author's view to name, and the argument should
say what the kind trait says. The derive rejects `output =` with an error
naming the right argument for the kind, as it rejects `response =` on a
stream, `item =` on a request, and either on a notification. The `Output = ()`
a notification needs is written by the derive; no author writes it.

### What generic code reads

Code generic over every kind — `Request<Op>`, `RequestHandle`, the bridge,
the router, middleware, type generation, the `#[effect]` macro's output —
keeps `Op: Operation` and keeps reading `Op::Output`. Type generation, the one
site that reads `Op::KIND`, reads `<Op::Kind as operation::Kind>::VALUE`
instead; the bridge takes a request's kind from its handle and does not
change. A hidden trait is still a trait: `use crux_core::capability::Operation`
in a middleware or a router of your own keeps compiling, and so does every
`<Op as Operation>::Output` written against it. What the hiding changes is the
documentation and the expectation, not the path.

Code generic over one kind is bounded on that kind's trait, as the parent
RFC's `Command` constructors already are, and reads the payload under the
kind's own word:

```rust
pub fn request_from_shell<Op>(operation: Op) -> RequestBuilder<Effect, Event, impl Future<Output = Op::Response>>
where
    Op: operation::Request,
    Effect: From<Request<Op>>;

pub fn stream_from_shell<Op>(operation: Op) -> StreamBuilder<Effect, Event, impl Stream<Item = Op::Item>>
where
    Op: operation::Stream,
    Effect: From<Request<Op>>;
```

Because the kind trait pins `Output` to its payload, a `Request<Op>` built
inside `request_from_shell` holds a `RequestHandle<Op::Output>` that the
compiler knows is a `RequestHandle<Op::Response>`; the two spellings name one
type, and no conversion is needed where a kind-specific constructor hands a
container to the kind-neutral machinery. Under an `operation::Stream` bound,
`Op::Output` still compiles and still names the item, but `Op::Item` is the
spelling to prefer.

One method on `Operation` has callers outside generic code:
`register_types_facet`, which a capability's typegen support calls by name —
`HttpRequest::register_types_facet(registry)` in `crux_http`, and its
siblings in `crux_kv` and `crux_time` — and which the shipped shell handlers
RFC coerces to a plain function pointer. A method on a hidden trait is an
awkward thing to document a call to. The proposal is a provided method of the
same name and signature on each public kind trait, delegating to the hidden
one, so that `Get::register_types_facet` reads as a method of
`operation::Request` and keeps compiling. Whether that or a free function in
`type_generation::facet` is the better home is an open question below.

### What this settles

Five review threads on the compat stack are answered by this shape, and each
answer follows from it rather than being a separate decision.

- The request's payload type moves to the `Request` trait, as asked on #588,
  and is called `Response` there.
- A stream's payload is `type Item`, and the derive's argument is `item =`, as
  asked on #583. The parent RFC's concession that a stream had to use `Output`
  dissolves: the uniform name generic code needs is `Output` on the
  supertrait, so the stream trait is free to use the word that reads best.
- `Output = !` for notifications, asked on #581 and #583, is no longer a
  question. `operation::Notify` declares no payload type of its own; the `()`
  is `Output` on the supertrait, written by the derive and read only by
  generic code.
- The inconsistency review of #583 objected to is not merely checked but
  unwritable, because only the derive writes the declaration — which is what
  that review said removing `Operation` would achieve, achieved without
  removing it.
- `const KIND` and its post-monomorphisation `const` assertion go, so the
  wrong constructor is visible to `cargo check` and the editor, as the parent
  RFC promised.

## Drawbacks

**No hand-written operations.** An author who wants to see or control what the
derive writes cannot, short of implementing a trait whose path tells them not
to. The derive handles generics and `where` clauses, and foreign types have to
be wrapped regardless, so nothing is lost in what can be expressed; what is
lost is a teaching device — the book has always shown `impl Operation for` to
say what an operation is — and the option of not depending on a proc macro.
The book can show the derive's expansion instead, and every Crux app already
depends on `crux_macros` for `#[effect]`.

**`crux_macros` becomes a required dependency.** It is optional today, behind
`crux_core`'s default feature. A trait only the derive implements cannot be
behind a feature that turns the derive off. No crate in this repository
disables the default features, and an app cannot declare an effect without
the macro crate, so this formalises what is already true.

**The hidden trait still exists, and so does `Output = ()`.** A notification's
supertrait impl carries a unit type that nothing reads, and a reader who opens
the derive's expansion sees a trait the documentation does not show. Both are
the price of `Request<Op>` holding notifications in the same container as
everything else, and both are written by the derive and read by nothing an
author writes.

**Sealing is by convention.** A hidden module and a marker trait stop nobody
who is determined. What they do is make the unsupported path visibly
unsupported, which is what the language allows.

**`output =` renames for every derived request.** The compat release's
`#[operation(request, output = T)]` becomes `#[operation(request, response = T)]`,
which touches every request in `crux_kv`, `crux_time` and the `weather`
example, and every request in an application written against the compat
release. It is a rename the derive reports with the new spelling, so nothing
compiles with a silently wrong meaning; but it is a rename the previous draft
of this RFC did not ask for.

**Every operation must declare a kind.** This is the parent RFC's breaking
change, not a new one — `type Kind` cannot default — but this RFC is where
it lands. Migration says what that costs and how to pay it.

## Migration

This is a breaking change, and it lands with the rest of the parent RFC's
breaking stage. What it costs, by who pays:

**Derive users.** A notification does not change. A request renames
`output =` to `response =`, a stream renames `output =` to `item =`; the
derive rejects the old spelling with a message naming the new one, so nothing
compiles with a silently wrong meaning. In this repository that is every
derived request — `crux_kv/src/operation.rs`, `crux_time/src/operation.rs`,
the `weather` example's `location` and `secret` effects — and the one derived
stream, the `notes` example's `Subscribe`.

**Bounds.** `Op: operation::Notify`, `Op: operation::Request` and
`Op: operation::Stream` do not change. `Op::Output` still compiles under any of
them; under `operation::Request` the payload can now be spelled
`Op::Response`, and under `operation::Stream` it reads better as `Op::Item`.

**Code generic over `Operation`.** `Op::Output` does not change. `Op::KIND`
becomes `<Op::Kind as operation::Kind>::VALUE`; in `crux_core` that is type
generation's `EffectBuilder::variant`, and outside it, any middleware or
router that read the const, which nothing in this repository does. Third-party
middleware and routers written against `<Op as Operation>::Output` compile
unchanged; the trait is hidden from the documentation, not from the compiler.

**Hand-written `impl Operation` blocks.** They stop compiling — a hidden
supertrait they do not implement, a `Kind` they do not name — and the recipe
is the same for all of them: delete the impls, derive. In this repository the
two that declare a kind are `crux_http`'s `HttpRequest`
(`crux_http/src/protocol.rs`) and `crux_core`'s own `RenderOperation`:

```rust
// compat release
impl Operation for RenderOperation {
    type Output = ();
    const KIND: Option<OperationKind> = Some(OperationKind::Notify);
}
impl operation::Notify for RenderOperation {}

// breaking release
#[derive(Operation, Facet, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[operation(notify)]
pub struct RenderOperation;
```

**Hand-written `impl Operation` blocks that declared no kind.** These are the
larger group, because the compat release let them be and the breaking release
cannot. The recipe is to look at how the operation is sent and declare that:
an operation only ever passed to `request_from_shell` becomes
`#[operation(request, response = <its Output>)]`. In this repository they are
the eight the parent RFC counted — `doctest_support/src/{basic_delay,delay,lib}.rs`,
`examples/counter-http/shared/src/sse.rs`, and `capabilities/{mod,sse}.rs` in
`counter-middleware` and `counter-routing` — and the operations in
`crux_core`'s own tests, each of which is sent one way.

Except that not all of them are. The parent RFC said it looked for an
operation sent through two different constructors and found none; this RFC's
exploration found them. `crux_core/tests/middleware.rs` declares one
`RandomNumberRequest`, sends it with `request_from_shell` for the first dice
and `stream_from_shell` for the rest, and resolves it from one middleware
either way. Four of the doctests on `command/builder.rs` send one
`AnOperation` through `request_from_shell` and then `notify_shell`, or a
request and then a stream, or a stream and then a request. A type has one
`Kind`, so each of these splits into two operation types, one per kind, and
the middleware or the effect enum gains a variant. None of these is library
code, and the pattern appears nowhere in the capability crates or the
examples' applications, but the claim that it did not exist was wrong, and
an application that leaned on the legacy flexibility in the same way will
have to split its operation too.

**`default-features = false`.** An application that disabled `crux_core`'s
default features to drop the macros loses them anyway: the breaking release
makes `crux_macros` a plain dependency. Nothing in this repository does this.

**The transitional machinery goes.** Each of these exists only because the
compat release checks the kind after monomorphisation, or lets an operation
declare none, or lets an author write the declaration by hand:

- `crux_core/tests/trybuild.rs`, whose module doc explains that the `fail/`
  cases are post-monomorphisation and so need `cargo build`, and why their
  `.stderr` snapshots depend on `rust-src`.
- The four `tests/ui/fail` cases that snapshot the E0080 assertion message:
  `derive_notify_requested`, `notify_op_requested`, `request_op_notified` and
  `request_op_streamed`. The fifth, `notify_marker_needs_unit_output`, held a
  claim about hand-written impls — a notification whose payload is not `()`
  does not compile — that the derive makes unwritable, so it goes too.
- `tests/ui/pass/legacy_operation_takes_any_constructor.rs` and
  `an_operation_without_a_kind_takes_any_constructor` in
  `crux_core/src/command/tests/basic_effects.rs`, which pin the behaviour of
  an operation that declares no kind.
- The legacy no-kind handling. In the bridge, the documented fallback to "the
  kind the call site chose" for an operation that declares none, which is why
  the registry takes a request's kind from the handle it is given rather than
  from the operation type. In type generation, the `Option<OperationKind>` on
  each effect variant, `is_legacy`, and the handler method that hands a raw
  request id and a `resolve` callback to a shell for an operation whose kind
  is unknown.

The [migration guide](../guide/migrate-per-operation-types.md#coming-in-the-breaking-release)
carries the user-facing version of this list.

## Alternatives considered

**Remove `Operation` outright.** `Notify`, `Request` and `Stream` as the only
traits, with `Notify` carrying no associated type at all — the shape
suggested in review of the derive, and the question this RFC's exploration
set out to answer. The Design section gives the three routes to it that Rust
rejects: blanket impls of a common trait from the three kind traits overlap;
a common trait on the container, written by the derive, is an orphan impl;
and erasing the payload inside the container leaves the bridge nothing to
deserialise with. One route works, and it was carried far enough to compile:

```rust
pub mod operation {
    pub trait Notify: Send + 'static {}
    pub trait Request: Send + 'static { type Response: Send + Unpin + 'static; }
    pub trait Stream: Send + 'static { type Item: Send + Unpin + 'static; }
}

// Three containers, one per kind, in place of today's Request<Op>.
pub struct Notification<Op: operation::Notify> { pub operation: Op }
pub struct Request<Op: operation::Request> { pub operation: Op, pub handle: RequestHandle<Op::Response> }
pub struct Subscription<Op: operation::Stream> { pub operation: Op, pub handle: RequestHandle<Op::Item> }

// The unifying trait, on the three containers crux_core owns. Sealed;
// implemented exactly three times, so it cannot overlap and nobody else
// writes it.
pub trait Envelope: sealed::Sealed + Send + 'static {
    type Op: Send + 'static;
    type Output: Send + Unpin + 'static;
    const KIND: OperationKind;
    fn split(self) -> (Self::Op, RequestHandle<Self::Output>);
}
```

Generic code bounds on `E: Envelope` instead of `Op: Operation`, and the
`#[effect]` macro learns each variant's kind from the container the author
names in the enum:

```rust
// The `notes` example's effect enum, in this shape.
#[effect(facet_typegen)]
pub enum Effect {
    Render(Notification<RenderOperation>),
    Publish(Notification<pub_sub::Publish>),
    Subscribe(Subscription<pub_sub::Subscribe>),
    KvGet(Request<kv::Get>),
    KvSet(Request<kv::Set>),
    TimeNotifyAfter(Request<time::NotifyAfter>),
    TimeClear(Request<time::Clear>),
}
```

It has real virtues. There is no hidden trait and no `Output = ()`; the
parent RFC's `type Kind`, sealed `Kind` trait and unit types are not needed,
because the container is the kind; `Request<Op>` implements `Resolvable` and
`Notification<Op>` does not, so resolving a notification stops compiling
instead of failing at run time; and an effect enum documents, at a glance,
which of its variants fire and forget, which are answered once and which
stream. The container for a stream is called `Subscription` rather than
`Stream` because capability code imports `futures::Stream` in the same
module — the `notes` example's pub/sub capability already does — and the
collision would be constant.

What it costs is why it is rejected. The kind is declared twice, in the
derive and in the enum, which is the shape this whole design exists to
remove; a mismatch is caught, but an application author writing
`Clear(???<crux_time::operation::Clear>)` has to know whether `Clear` is a
request before they can write the variant. Every effect enum ever written
changes: in this repository, thirty-seven in code and forty-two in doctests.
`capability::Operation` and `Op::Output` go, which this RFC promised third
parties they would keep. `Request<Op>` gaining a struct bound ripples the
container into every generic parameter that held an operation —
`ResolveSink<Op>` becomes `ResolveSink<Request<Op>>`, likewise `Parked` and
the router's `Registry` — and `register_types_facet` leaves the operation for
the container. Stream and notification capabilities change their public
bounds, from `From<Request<Subscribe>>` to `From<Subscription<Subscribe>>`.
A reproduction of the design compiles and runs; an estimate against this
repository put the change at over a hundred files. And the decisive point is
the first one: the effect enum should not have to say what kind an operation
is, because the operation already says so.

**A hidden `Kinded` trait written by the derive, to keep `Http(HttpRequest)`
while removing `Operation`.** `trait Kinded { type Envelope: Envelope<Op = Self>; }`,
so that the macro writes `Http(<HttpRequest as Kinded>::Envelope)`. This is
the supertrait under another name: one trait every operation implements once,
written by the derive, unable to be blanket-implemented from the kind traits
for the same overlap. It relocates `Output` from the operation to the
container and drops `Output = ()` from notifications, which is a naming
improvement, not a structural one, and it keeps every cost of the three
containers. Keeping `Operation`, hidden, is the same shape with less churn.

**Tied bounds, hand-written impls allowed.** The previous draft's lean: the
shape above with `Operation` public, an author free to write both impls. It
has the authored diagnostic and the right payload words, and it reintroduces
the two things the derive-only shape removes: a disagreement between the
impls, which is at least a compile error at the declaration, and the orphan
— a `Kind = kind::Request` with no `impl operation::Request`, which compiles,
satisfies every generic site, cannot be sent, and is then told it is not a
request. Both are only reachable by hand, and the derive-only shape closes
the hand-written path for exactly that reason.

**Blanket impls.** The parent RFC's shape, carrying the payload:
`impl<Op: Operation<Kind = kind::Request>> Request for Op { type Response = Op::Output; }`
and likewise for the other two. An author writes one impl, and a disagreement
or an orphan cannot be written. But the payload word the author writes is
`Output`, which is the word this RFC is trying to get rid of at the
declaration; and the wrong constructor is an E0271 that
`#[diagnostic::on_unimplemented]` cannot decorate, so the author reads the
compiler's walk through the blanket impl's bounds — which does point at their
own `type Kind` line, and then keeps going. The derive-only shape has
everything this shape has and the authored message besides.

**A payload-free `Operation`.** Keep the supertrait for the kind and the
typegen hooks, but with no payload type on it. This fails where removing the
trait fails: `Request<Op>` and the macro's kind-agnostic calls need to name
the payload from `Op: Operation`, and a supertrait without a payload gives
them nothing to name.

**The names the other way round.** Rename `Operation::Output` to `Response`,
as the kind-neutral word, and give `operation::Request` a `type Output`. It
reads well at the declaration site, but it renames the one associated type
that every generic site in `crux_core` and every third-party middleware or
router spells, for no gain in what the compiler can check. Keeping `Output` on
the supertrait and putting `Response` on the request trait is the same shape
with that migration cost removed, and "response" is already the word the
bridge's errors use.

**Leave `Notify`'s payload free.** `operation::Notify: Operation<Kind =
kind::Notify>` without `Output = ()`, since nothing reads a notification's
payload. With the derive the only author it would make no difference to what
is written, but it would leave the derive picking a type for no reason and
generic code unable to rely on one. Pinned is proposed.

**Keep `const KIND` and the assertion permanently.** The parent RFC's
transitional shape could stay. A defaulted const is the only stable way for
undeclared operations to coexist with declared ones, and the
`const { assert!(..) }` does reject the wrong constructor. But the error only
appears on `cargo build`, never in `cargo check` or the editor; its rendering
depends on whether `rust-src` is installed, which is why `trybuild.rs` has a
module doc about it; and the const and the marker are two declarations that a
hand-written impl can make disagree. Once the legacy default is gone there is
no reason to keep any of it.

## Open questions

1. **Sealing by convention, or not at all.** `#[doc(hidden)]` on `Operation`
   with the derive documented as the only way to declare an operation may be
   enough; the `__private::Sealed` supertrait adds one impl to the derive's
   output and one unmistakable signal to anyone who writes the trait by hand.
   The RFC proposes both. A reviewer who finds the marker theatrical should
   say so; nothing else in the design depends on it.
2. **Where `register_types_facet` lives for callers.** Provided methods on
   the three public kind traits, delegating to the hidden one, keep
   `HttpRequest::register_types_facet` compiling and reading naturally. A free
   function `type_generation::facet::register_operation::<Op>(registry)` is
   the other choice, one name instead of three, at the cost of every call site
   in the capability crates and the shipped shell handlers RFC changing.

## Next steps

1. Land the shape in the breaking release: the hidden, sealed `Operation` with
   `type Kind`; the kind traits with `Response` and `Item` and their
   `#[diagnostic::on_unimplemented]` messages; the `Command` and
   `CommandContext` bounds on the kind traits; `crux_macros` as a required
   dependency of `crux_core`.
2. The derive emits three impls from `#[operation(notify)]`,
   `#[operation(request, response = ..)]` and `#[operation(stream, item = ..)]`,
   and rejects `output =` with the right word for the kind.
3. Replace `Op::KIND` with `<Op::Kind as operation::Kind>::VALUE` in type
   generation, and spell the payload `Op::Response` and `Op::Item` where a
   kind-specific bound is already in place. Give `register_types_facet` its
   public home, per open question 2.
4. Migrate `HttpRequest` and `RenderOperation` to the derive; rename
   `output =` in `crux_kv`, `crux_time` and the `weather` example; declare a
   kind on the kind-less operations in `doctest_support`, the remaining
   examples and `crux_core`'s tests, splitting `RandomNumberRequest` in the
   middleware test and `AnOperation` in the builder doctests into one type per
   kind — alongside the parent RFC's other breaking-stage items: removing the
   deprecated enum APIs and re-exporting `store::KeyValue` and `clock::Time`
   at the crate roots.
5. Delete the `const` assertion and the transitional machinery listed under
   Migration.
6. Update the [migration guide](../guide/migrate-per-operation-types.md) and
   the [capabilities chapter](../part-2/capabilities.md) from "what the breaking
   release will do" to what it does, showing the derive's expansion where the
   chapter shows a hand-written impl today.
