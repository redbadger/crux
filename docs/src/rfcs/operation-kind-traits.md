# RFC: One trait per operation kind

```admonish
This RFC is **proposed**. It builds on the
[per-operation types RFC](./per-operation-types.md) and supersedes that RFC's
breaking stage — the target shape its Design section originally specified,
reproduced below under [What the parent proposed](#what-the-parent-proposed)
— and it is not yet implemented. Both shapes it compares have been compiled
and checked on stable Rust, but nothing here has shipped; if accepted, it
would land in the next major release of `crux_core`.
```

This RFC proposes that the three operation kinds — notify, request and stream
— each become a trait that names the payload an operation of that kind is
answered with, under the word that fits the kind: a request's `Response`, a
stream's `Item`, and nothing for a notification. `Operation` survives as the
supertrait of all three, keeping the kind as an associated type and its
existing `Output`, which the machinery generic over every kind reads and which
the kind traits are bound to, so the two can never disagree. Two ways of
writing that bound are compared, and choosing between them is the question
this RFC most wants answered.

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
associated type, and have each kind trait name the payload under its own word,
bound to `Output` so that the two are one declaration:

```rust
pub trait Operation: Send + 'static {
    type Kind: operation::Kind;
    type Output: Send + Unpin + 'static; // unchanged from today
}

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

With the bounds written this way — the *tied* shape — an operation is two
impls, which the derive writes: `Operation`, naming its `Kind` and `Output`,
and one kind trait, naming the same payload under the kind's own word.

```rust
impl Operation for Get {
    type Kind = operation::kind::Request;
    type Output = ValueResult;
}

impl operation::Request for Get {
    type Response = ValueResult;
}
```

The kind traits can instead be blanket-implemented from `Kind`, reading the
payload off `Operation` — the *blanket* shape — in which case an operation
is the first impl alone and the kind trait is derived, never declared.
[Two ways to bind the kind traits](#two-ways-to-bind-the-kind-traits) compares
them, with the compiler's output for each mistake an author can make. This RFC
leans towards the tied shape, for one reason above the others: with it, the
wrong `Command` constructor is an unimplemented trait, so
`#[diagnostic::on_unimplemented]` fires and the author reads an authored
message; with the blanket shape it is an associated-type mismatch, the
attribute does not fire, and the author reads the compiler's walk through the
bounds. The choice is open.

Either way, three things follow. The wrong constructor is a trait-bound error
that `cargo check` and the editor report, rather than the compat release's
post-monomorphisation `const` assertion. The payload has the right name where
an author writes it and where kind-specific code reads it. And `Request<Op>`,
the bridge registry, the effect router, middleware and type generation keep
reading `Op::Output` from a single `Op: Operation` bound, exactly as they do
today, so none of them changes.

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
once and the marker follows. That much is kept here, as one of the two shapes
compared. What it left in place is the payload.

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

## Goals

1. The name an author writes for an operation's payload is the right one for
   its kind: `Response` for a request, `Item` for a stream, nothing for a
   notification.
2. An operation's kind and payload cannot be declared inconsistently. Any
   disagreement between the `Operation` impl and the kind trait impl is a
   compile error at the declaration, not at a use site — or, in the blanket
   shape, cannot be written at all.
3. The wrong `Command` constructor is a trait-bound error, visible to
   `cargo check` and the editor, and as far as the compiler allows it names
   the right constructor.
4. The machinery generic over every kind — `Request<Op>`, the bridge registry,
   the effect router, middleware, type generation, the `#[effect]` macro's
   generated helpers — keeps a single `Op: Operation` bound and keeps reading
   `Op::Output`, so it does not change.
5. Code written against the compat release's bounds and derive changes as
   little as possible, and every change is a rename.

## Non-goals

- Changing the wire format, the generated shell API, or anything a shell
  sees. The kind reaches shells the way it does today.
- Redesigning `Command`, the effect router or middleware. The `Command` and
  `CommandContext` bounds tighten to the kind traits, as the parent RFC
  proposed. Nothing else changes.
- Using the never type. `operation::Notify` declaring no payload type of its
  own is what makes `Output = !` unnecessary; this RFC does not propose
  adopting `!` when it becomes available.
- Removing `Operation`. The alternatives section explains why not.

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
staying on `Operation`. What changes is the three kind traits, which stop
being payload-free markers and carry the payload type under the kind's own
word. The concession about `type Item` dissolves, because the uniform name
generic code needs is `Output` on the supertrait, and the kind trait is free
to use the word that reads best.

### The shape

```rust
pub trait Operation: Send + 'static {
    /// How many times this operation expects to be resolved, as a type.
    type Kind: operation::Kind;

    /// The value the shell resolves with, named without reference to the
    /// kind: `()` for a notification, a request's `Response`, a stream's
    /// `Item`. Generic code reads this one. An author writes the kind
    /// trait's word for it, and the derive writes both.
    type Output: Send + Unpin + 'static;

    // register_types_facet as today, bounded on Self::Output
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
two impls one declaration: implementing `operation::Request` for a type
obliges its `Operation` impl to say `Kind = kind::Request` and
`Output = <that type's Response>`, and the compiler checks the obligation
where the kind trait is implemented.

`Notify` pins `Output = ()`. Nothing reads a notification's payload, so the
type could be left free, but pinning it gives a hand-written notification one
right answer and keeps the compat release's guarantee that a notification
cannot claim a payload. It is the price of `Request<Op>` holding notifications
in the same container as everything else, and it is one line the derive
writes.

`const KIND` goes, and with it its `Option`: every operation declares a kind,
because an associated type cannot default. `<Op::Kind as Kind>::VALUE` is the
static kind, readable in any generic context that has `Op: Operation`, which
is what `Op::KIND` gave type generation in the compat release.

The `#[diagnostic::on_unimplemented]` messages are the static form the
attribute allows. It accepts only `message`, `label` and `note`, and
interpolates only the type itself, `{Self}`; it rejects conditional clauses
and associated-type interpolation, so a message cannot say what kind the type
*is*, only what it is not. "`Get` is not a notification", with a note naming
the two constructors that send the other kinds, is what stable Rust can
produce, and it is quoted from the compiler below.

### Why `Operation` survives as a supertrait

The obvious alternative, once each kind has a trait of its own, is to remove
`Operation` altogether and leave `Notify`, `Request` and `Stream` as the only
traits, with `Notify` having no associated type at all. Review of the derive
(#583) suggested exactly that, and in the same breath saw the difficulty:

> not sure how we're going to write the trait bound for a function which
> takes an effect and generates it's ID depending on the kind if we remove
> `Operation` later. Maybe something like a blanket impl of a trait for each
> of the three subtraits?

It does not work, for one concrete reason. `crux_core::Request<Op>` is the
container every effect variant holds, for all three kinds — notifications
included, since `notify_shell` builds one with `Request::resolves_never`. It
names the payload type from a single bound:

```rust
pub struct Request<Op>
where
    Op: Operation,
{
    pub operation: Op,
    pub handle: RequestHandle<Op::Output>,
}
```

So does everything downstream of it. The bridge deserialises a response into
`Op::Output` before it reaches the handle; the effect router's registry parks
requests in a `Storage<Op::Output>` and middleware's
`EffectResolver<<Op as Operation>::Output>` names the same type; type
generation's `EffectBuilder::variant::<Op>` reflects `Op::Output` for the
shell; `Operation::register_types_facet` bounds it; and the `#[effect]` macro
generates `resolve_*` and `expect_*` helpers whose signatures spell
`<Op as Operation>::Output`. Each of these is written once, over `Operation`,
and is correct for every kind. Third-party middleware and routers are written
the same way.

Without `Operation`, or with an `Operation` that has no payload type,
`Request<Op>` has nothing to name. Every one of those sites would have to
become three — one per kind trait — or be rewritten around an enum of the
three, which is the coarseness this whole design set out to remove. That is
the delicate part of the codebase, and both shapes proposed here are chosen so
that it does not change at all.

The blanket impl of a common trait from the three kind traits, the shape the
review comment reached for, is what Rust rejects:

```rust
impl<Op: operation::Request> Operation for Op { type Output = Op::Response; }
impl<Op: operation::Stream> Operation for Op { type Output = Op::Item; }
impl<Op: operation::Notify> Operation for Op { type Output = (); }
```

The three impls overlap — nothing proves that a type cannot implement two of
the kind traits — and coherence does not accept a proof that the kinds are
disjoint, because there is no way to state one. So the supertrait keeps its
one kind-neutral associated type, `Output`, and the kind traits are bound to
it rather than the other way round.

### Two ways to bind the kind traits

Both shapes keep `Operation { type Kind; type Output }`, both give the kind
traits their payload words, and both compile against a `Request<Op>` generic
over all three kinds, constructors bounded on the kind traits, and an
operation of each kind. They differ in who writes the kind trait impl.

**Tied bounds** — the shape above. The kind trait's supertrait bound pins
`Operation`'s `Kind` and `Output`; an author (or the derive) writes both impls.

**Blanket impls** — the parent RFC's shape, carrying the payload. The kind
trait is implemented for every `Operation` with the matching `Kind`, reading
the payload off the supertrait; an author writes only the `Operation` impl.

```rust
pub trait Request: Operation<Kind = kind::Request> {
    type Response: Send + Unpin + 'static;
}

impl<Op: Operation<Kind = kind::Request>> Request for Op {
    type Response = <Op as Operation>::Output;
}

// and likewise for Notify (with Output = ()) and Stream (type Item)
```

The comparison, with the compiler's output for each mistake. The transcripts
are from rustc 1.98.1, edition 2021, abridged to the lines that matter. They
come from a standalone reproduction of each shape — the traits, a
`Request<Op>` generic over all three kinds, constructors bounded on the kind
traits, and one operation of each kind — compiled outside the repository, so
the file names and line numbers they cite are that reproduction's, not
anything in `crux_core`.

**Impls per operation.** Tied: two, which the derive writes. Blanket: one; the
kind trait is derived, never declared.

**Disagreement between the two.** Tied: possible to write, rejected where the
kind trait is implemented, pointing at both lines:

```text
error[E0271]: type mismatch resolving `<Get as Operation>::Kind == Request`
  --> sw_tied_mismatch.rs:62:29
   |
62 | impl operation::Request for Get { type Response = String; }
   |                             ^^^ type mismatch resolving `<Get as Operation>::Kind == Request`
   |
note: expected this to be `kind::Request`
  --> sw_tied_mismatch.rs:61:38
   |
61 | impl Operation for Get { type Kind = kind::Notify; type Output = String; }
   |                                      ^^^^^^^^^^^^
note: required by a bound in `operation::Request`
  --> sw_tied_mismatch.rs:32:34
   |
32 |     pub trait Request: Operation<Kind = kind::Request, Output = <Self as Request>::Response> {
   |                                  ^^^^^^^^^^^^^^^^^^^^ required by this bound in `Request`
```

A payload that differs between the two impls fails the same way, resolving
`<Get as Operation>::Output == String`. Blanket: unrepresentable. There is no
second impl to disagree.

**The orphan.** Tied: a type may declare `type Kind = kind::Request` and never
implement `operation::Request`. It satisfies every `Op: Operation` site —
`Request<Op>`, the registries, type generation — and no constructor can send
it, and when someone tries, the error says "`Get` is not a request", which the
type's own impl claims it is. The derive always writes both impls, so only a
hand-written operation can be an orphan, but nothing stops one. Blanket:
cannot happen; the kind trait follows from `Kind`.

**The wrong constructor.** This is the mistake the whole design exists to
catch, and it is where the two shapes part. Tied: an unimplemented trait,
E0277, so `#[diagnostic::on_unimplemented]` fires and the author reads the
message the trait carries:

```text
error[E0277]: `Get` is not a notification
  --> sw_tied_diag.rs:78:26
   |
78 |     let _ = notify_shell(Get); // wrong constructor
   |             ------------ ^^^ not a notification
   |             |
   |             required by a bound introduced by this call
   |
help: the trait `operation::Notify` is not implemented for `Get`
  --> sw_tied_diag.rs:65:1
   |
65 | pub struct Get;
   | ^^^^^^^^^^^^^^
   = note: a request is sent with request_from_shell, a stream with stream_from_shell
help: the trait `operation::Notify` is implemented for `Publish`
```

Blanket: an associated-type mismatch, E0271, because `Notify` *is*
implemented for every `Operation` and the failure is in the blanket impl's
where-clause. The same attribute is present and does not fire; the author
reads the compiler's walk through the bounds instead:

```text
error[E0271]: type mismatch resolving `<Get as Operation>::Kind == Notify`
  --> sw_blanket_diag.rs:73:26
   |
73 |     let _ = notify_shell(Get); // wrong constructor
   |             ------------ ^^^ type mismatch resolving `<Get as Operation>::Kind == Notify`
   |             |
   |             required by a bound introduced by this call
   |
note: expected this to be `kind::Notify`
  --> sw_blanket_diag.rs:64:38
   |
64 | impl Operation for Get { type Kind = kind::Request; type Output = String; }
   |                                      ^^^^^^^^^^^^^
note: required for `Get` to implement `operation::Notify`
  --> sw_blanket_diag.rs:31:59
   |
31 |     impl<Op: Operation<Kind = kind::Notify, Output = ()>> Notify for Op {}
   |                        -------------------                ^^^^^^     ^^
   |                        |
   |                        unsatisfied trait bound introduced here
note: required by a bound in `notify_shell`
```

The blanket error has one thing the tied error lacks: it points at the
author's own `type Kind = kind::Request` line. The tied error has what the
parent RFC promised and what a newcomer needs: a sentence saying what is
wrong and a note naming the constructor to use.

**Opting out.** Tied: an author who wants to hand-write the kind impl does so;
it is the ordinary way to declare an operation without the derive. Blanket:
the kind impl cannot be written by hand at all, because the blanket impl
already covers the type:

```text
error[E0119]: conflicting implementations of trait `operation::Request` for type `Get`
  --> sw_blanket_optout.rs:64:1
   |
31 |     impl<Op: Operation<Kind = kind::Request>> Request for Op {
   |     -------------------------------------------------------- first implementation here
...
64 | impl operation::Request for Get { type Response = String; }
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ conflicting implementation for `Get`
```

This is not a defect in the blanket shape — there is nothing an author would
need to write — but it does mean the shape is closed to extension in a way the
tied one is not.

**Where this RFC leans.** Towards the tied bounds, because of the wrong
constructor. The parent RFC promises an authored message naming the right
constructor, the error in question is the one a first-time capability author
is most likely to hit, and only the tied shape can produce it on stable Rust.
Against that, the blanket shape delivers what the review of #583 asked for —
the inconsistency is impossible, not checked — and has no orphan. The derive
narrows the gap considerably, since derived operations cannot disagree or be
orphaned in either shape, which leaves the choice resting on hand-written
operations and on the diagnostic. This is the question the RFC most wants a
reviewer's answer to.

### What the derive emits

`#[derive(Operation)]` writes both impls in the tied shape, and the one impl
in the blanket shape. Its arguments become:

```rust
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
#[operation(notify)]
pub struct Publish(pub Vec<u8>);
// impl Operation { Kind = kind::Notify; Output = (); }
// impl operation::Notify {}                        (tied shape only)

#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
#[operation(request, output = ValueResult)]
pub struct Get { pub key: String }
// impl Operation { Kind = kind::Request; Output = ValueResult; }
// impl operation::Request { Response = ValueResult; }  (tied shape only)

#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
#[operation(stream, item = Message)]
pub struct Subscribe;
// impl Operation { Kind = kind::Stream; Output = Message; }
// impl operation::Stream { Item = Message; }         (tied shape only)
```

`item =` on a stream is the one change to the derive's surface. A request
keeps `output =`, which still names a type literally called `Output` on the
operation's `Operation` impl; whether the derive should also, or instead,
accept `response =` is an open question below. `output =` on a stream, and
`item =` on a request, are errors that name the right argument, as an
`output =` on a notification is today. The `Output = ()` a notification needs
is written by the derive; no author writes it unless they write the impls by
hand.

### What generic code reads

Code generic over every kind — `Request<Op>`, `RequestHandle`, the bridge,
the router, middleware, type generation, the `#[effect]` macro's output —
keeps `Op: Operation` and keeps reading `Op::Output`. Type generation, the one
site that reads `Op::KIND`, reads `<Op::Kind as operation::Kind>::VALUE`
instead; the bridge takes a request's kind from its handle and does not
change. Code generic over one kind is bounded on that kind's trait, as the
parent RFC's `Command` constructors already are, and reads the payload under
the kind's own word:

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

### What this settles

Four review threads on the compat stack are answered by this shape, and each
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
- `const KIND` and its post-monomorphisation `const` assertion go, so the
  wrong constructor is visible to `cargo check` and the editor, as the parent
  RFC promised.

## Drawbacks

**Two impls per operation, in the tied shape.** The derive writes both, and a
hand-written operation was already two blocks in the compat release — the
`Operation` impl and the marker — so for anyone on the compat shape the count
does not change. For anyone still on a single legacy `impl Operation` with no
kind, it goes from one block to two, or to the derive. A reader of
hand-written code sees the payload named twice, as `Output` and as `Response`
or `Item`; the bound guarantees they agree, but they are two lines.

**The orphan, in the tied shape.** A hand-written `Operation` impl with
`type Kind = kind::Request` and no `impl operation::Request` compiles, passes
every generic site, and cannot be sent by any constructor; the diagnostic then
says it is not a request. Only hand-written operations can do this. The
blanket shape has no such case.

**No authored diagnostic, in the blanket shape.** The wrong constructor is an
E0271 the attribute cannot decorate, as the transcript above shows. It points
at the offending `type Kind` line, which helps, and then walks through the
blanket impl's bounds, which does not.

**Every operation must declare a kind.** This is the parent RFC's breaking
change, not a new one — `type Kind` cannot default — but this RFC is where
it lands. Migration says what that costs and how to pay it.

**`Output = ()` is still there.** A notification's supertrait impl carries a
unit type that nothing reads. It is one line the derive writes, and it is the
price of `Request<Op>` holding notifications in the same container as
everything else.

## Migration

This is a breaking change, and it lands with the rest of the parent RFC's
breaking stage. What it costs, by who pays:

**Derive users.** A request or a notification does not change. A stream
renames `output =` to `item =`; the derive rejects the old spelling with a
message naming the new one, so nothing compiles with a silently wrong meaning.
In this repository the only derived stream is the `notes` example's
`Subscribe`.

**Bounds.** `Op: operation::Notify`, `Op: operation::Request` and
`Op: operation::Stream` do not change. `Op::Output` still compiles under any of
them; under `operation::Request` the payload can now be spelled
`Op::Response`, and under `operation::Stream` it reads better as `Op::Item`.

**Code generic over `Operation`.** `Op::Output` does not change. `Op::KIND`
becomes `<Op::Kind as operation::Kind>::VALUE`; in `crux_core` that is type
generation's `EffectBuilder::variant`, and outside it, any middleware or
router that read the const, which nothing in this repository does. Third-party
middleware and routers written against `<Op as Operation>::Output` compile
unchanged.

**Hand-written `impl Operation` blocks that declared a kind.** `const KIND`
becomes `type Kind`, and in the tied shape the marker impl gains the payload
type. In this repository that is `crux_http`'s `HttpRequest`
(`crux_http/src/protocol.rs`) and `crux_core`'s own `RenderOperation`, a
hand-written notification:

```rust
// compat release
impl Operation for RenderOperation {
    type Output = ();
    const KIND: Option<OperationKind> = Some(OperationKind::Notify);
}
impl operation::Notify for RenderOperation {}

// breaking release, tied shape
impl Operation for RenderOperation {
    type Kind = operation::kind::Notify;
    type Output = ();
}
impl operation::Notify for RenderOperation {}
```

**Hand-written `impl Operation` blocks that declared no kind.** This is the
largest cost, because the compat release let them be and the breaking release
cannot. An unmigrated impl stops compiling with `E0046`, and one that set the
const also gets `E0438`; both point at the impl, not at a use site. As with
the transcripts above, this one is from a standalone reproduction compiled
outside the repository, and its file name is the reproduction's:

```text
error[E0046]: not all trait items implemented, missing: `Kind`
  --> legacy_unmigrated.rs:17:1
   |
11 |     type Kind: Kind;
   |     --------------- `Kind` from trait
...
17 | impl Operation for Legacy {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^ missing `Kind` in implementation

error[E0438]: const `KIND` is not a member of trait `Operation`
  --> legacy_unmigrated.rs:25:5
   |
25 |     const KIND: Option<OperationKind> = Some(OperationKind::Request);
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ not a member of trait `Operation`
```

The recipe is to look at how the operation is sent and declare that. An
operation only ever passed to `request_from_shell` adds
`type Kind = operation::kind::Request;` and, in the tied shape,
`impl operation::Request for It { type Response = <its Output>; }` — or
switches to `#[derive(Operation)]` with `#[operation(request, output = ..)]`,
which is shorter and cannot be got wrong. An operation sent through two
different constructors has to split into two types, one per kind, because a
type has one `Kind`. The parent RFC looked for such a case across the
repository and its examples and found none; the eight kind-less impls in this
repository — `doctest_support/src/{basic_delay,delay,lib}.rs`,
`examples/counter-http/shared/src/sse.rs`, and `capabilities/{mod,sse}.rs` in
`counter-middleware` and `counter-routing` — are each sent one way, and each
is the parent RFC's remaining-examples migration.

**The transitional machinery goes.** Each of these exists only because the
compat release checks the kind after monomorphisation, or lets an operation
declare none:

- `crux_core/tests/trybuild.rs`, whose module doc explains that the `fail/`
  cases are post-monomorphisation and so need `cargo build`, and why their
  `.stderr` snapshots depend on `rust-src`.
- The four `tests/ui/fail` cases that snapshot the E0080 assertion message:
  `derive_notify_requested`, `notify_op_requested`, `request_op_notified` and
  `request_op_streamed`. The fifth, `notify_marker_needs_unit_output`, holds
  a claim that survives — a notification whose payload is not `()` does not
  compile — but under either shape it is an ordinary error that `cargo check`
  reports, so it can become a compile-fail doctest on `operation::Notify`
  rather than a `trybuild` case.
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
traits, with `Notify` carrying no associated type at all — the shape suggested
in review of the derive. As the Design section explains, `Request<Op>` and
everything generic over it then have no single bound from which to name the
payload, and recovering one with blanket impls of `Operation` from the three
kind traits is rejected as overlapping. The generic machinery would have to be
written three times or built around an enum of the kinds. Keeping the
supertrait costs one associated type and one line per derived notification;
removing it costs the part of the codebase this design most wants to leave
alone.

**A payload-free `Operation`.** Keep the supertrait for the kind and the
typegen hooks, but with no payload type on it. This fails in the same place
and for the same reason: `Request<Op>` needs to name the type of its handle
from `Op: Operation`, and a supertrait without a payload gives it nothing to
name.

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
payload. It would let a hand-written notification declare any `Output`, which
the compat release's `notify_marker_needs_unit_output` test exists to forbid,
and the derive would still have to pick something. Pinned is proposed.

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

1. **Tied bounds or blanket impls.** The comparison above is the whole of the
   question: an authored diagnostic at the wrong constructor and an open door
   for hand-written impls, against an inconsistency that cannot be written and
   no orphan. The RFC leans tied. A reviewer who weighs the #583 request for
   impossibility above the diagnostic should say so; the derive, the generic
   machinery and everything a shell sees are the same in both.
2. **The derive's argument for a request.** `output =` is kept because it
   still names a type called `Output` and because it spares every derived
   request a rename. `response =` would match the trait. The derive could
   accept both, with one deprecated; or `output =` could stay as the one
   spelling that means "the payload, whatever this kind calls it".

## Next steps

1. Settle open question 1 in review, then land the shape in the breaking
   release: the traits, the derive's `item =` argument and (in the tied shape)
   its second impl, the `#[diagnostic::on_unimplemented]` messages, and the
   `Command` and `CommandContext` bounds on the kind traits.
2. Replace `Op::KIND` with `<Op::Kind as operation::Kind>::VALUE` in type
   generation, and spell the payload `Op::Response` and `Op::Item` where a
   kind-specific bound is already in place.
3. Migrate the two hand-written kind-declaring impls, `HttpRequest` and
   `RenderOperation`, and the eight kind-less impls in `doctest_support` and
   the remaining examples, alongside the parent RFC's other breaking-stage
   items: removing the deprecated enum APIs and re-exporting `store::KeyValue`
   and `clock::Time` at the crate roots.
4. Delete the `const` assertion and the transitional machinery listed under
   Migration.
5. Update the [migration guide](../guide/migrate-per-operation-types.md) and
   the [capabilities chapter](../part-2/capabilities.md) from "what the breaking
   release will do" to what it does.
