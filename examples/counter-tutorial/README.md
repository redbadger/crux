# Counter tutorial example

This is the end state of the tutorial in Part I ("Basics") of the
[Crux book](https://redbadger.github.io/crux/). The book's code listings are
included from here, so the tutorial and this code stay in step.

The shells are deliberately hand-written: each one has a `CoreWrapper` that
drives the BoltFFI `CoreFfi` by hand — serializing events, handling the
effects that come back and reading the view model — so you can see exactly how a shell and the core
talk to each other. That makes it a good way to learn, but it is not the
recommended way to write a shell.

[`examples/counter`](../counter) is the same app written the recommended way,
using the generated `Core` and the shipped capability handlers. Start there
when you're building a real app.

## Architecture

The `shared` directory is a crate that implements the shared crux core. It contains:

- An `Event` with three variants: `Increment`, `Decrement` and `Reset`
- A `Model` with a `count` field
- Tests that ensure events update the `Model` correctly and produce the desired
  effects.

## Shells

- SwiftUI (iOS/macOS) — `apple/`
- Android/Kotlin — `Android/`
- NextJS — `web-nextjs/`

## Running

1. Choose a shell you're interested in, i.e. `apple` or `Android`.
2. In the shell's directory, run `just doctor` to make sure you have the right
  tools installed
3. Run `just dev` to generate code and build that shell
4. For `apple` and `Android`, open the IDE (Xcode or Android Studio). For
  `web-nextjs`, run `just serve` in the shell directory.
