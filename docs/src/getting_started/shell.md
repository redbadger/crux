# The shell

Type generation gives each shell three things to build on:

- the app's types (`Event`, `Effect`, `ViewModel` and the rest),
- an `EffectHandler` interface, with one method per operation the app can ask
  for, and
- a `Core` class that runs the loop: it sends events to the Rust core, hands
  each effect to your `EffectHandler`, resolves the request with what your
  method returns, and publishes the new view model whenever the core asks for
  a render.

So a shell writes its UI and an `EffectHandler`. The counter's only effect is
`Render`, which `Core` handles itself, so its handler is empty.
[Who drives the loop](../part-2/shell.md#who-drives-the-loop) in Part II
explains how this fits together, and
[The generated Core](../part-4/typegen.md#the-generated-core) has the exact
shape in each language.

## Setting up the project

The quickest way to start is to copy the shell you want from
[`examples/counter`](https://github.com/redbadger/crux/tree/master/examples/counter).
Each one has a `Justfile`: `just dev` runs the codegen, packages the core with
BoltFFI, and builds the shell. If you'd rather set the project up from
scratch, Part I's shell chapters walk through it for
[iOS/macOS](../part-1/shell/apple/index.md),
[Android](../part-1/shell/android/index.md) and
[React](../part-1/shell/web/react.md). Their project setup applies here too,
with two differences, because the generated package now depends on the
BoltFFI one:

- In `apple/project.yml`, the app target needs only the generated `App`
  package. It reaches the BoltFFI `Shared` package through `App`.
- On the web, run `boltffi pack wasm` _before_ the TypeScript codegen, because
  the generated package installs the wasm one as a dependency.

The code that differs is what follows.

## iOS/macOS

The handler conforms to the generated `EffectHandler` protocol:

```swift
// Swift: apple/CounterApp/CounterHandler.swift
{{#include ../../../examples/counter/apple/CounterApp/CounterHandler.swift}}
```

The app builds a `Core` from it, keeps it in `@State`, and puts it in the
environment:

```swift
// Swift: apple/CounterApp/CounterApp.swift
{{#include ../../../examples/counter/apple/CounterApp/CounterApp.swift}}
```

`Core` is `@Observable`, so a view that reads `core.view` redraws when the view
model changes. The buttons send events with `core.update`:

```swift
// Swift: apple/CounterApp/ContentView.swift
{{#include ../../../examples/counter/apple/CounterApp/ContentView.swift:content_view}}
```

## Android

The handler implements the generated `EffectHandler` interface:

```kotlin
// Kotlin: Android/app/src/main/java/com/crux/examples/counter/CounterHandler.kt
{{#include ../../../examples/counter/Android/app/src/main/java/com/crux/examples/counter/CounterHandler.kt}}
```

`Core` needs a `CoroutineScope` on the main dispatcher, and an Android
`ViewModel` provides one that lives as long as the screen does:

```kotlin
// Kotlin: Android/app/src/main/java/com/crux/examples/counter/CounterViewModel.kt
{{#include ../../../examples/counter/Android/app/src/main/java/com/crux/examples/counter/CounterViewModel.kt}}
```

`Core` publishes the app's `ViewModel` (the Crux one) as a `StateFlow`, which
Compose collects:

```kotlin
// Kotlin: Android/app/src/main/java/com/crux/examples/counter/MainActivity.kt
{{#include ../../../examples/counter/Android/app/src/main/java/com/crux/examples/counter/MainActivity.kt:activity}}
```

## Web (TypeScript)

The handler implements the generated `EffectHandler` interface, and
`Core.create` builds the core from it, with a callback for each new view
model. It's an `async` factory, because the wasm module loads asynchronously:

```typescript
// TypeScript: web-nextjs/src/app/core.ts
{{#include ../../../examples/counter/web-nextjs/src/app/core.ts}}
```

The page creates the core once, passing React's `setView` as the callback:

```typescript
// TypeScript: web-nextjs/src/app/page.tsx
{{#include ../../../examples/counter/web-nextjs/src/app/page.tsx:create_core}}
```

## Windows (C#)

The handler implements the generated `IEffectHandler` interface:

```csharp
// C#: windows/CounterApp/CounterHandler.cs
{{#include ../../../examples/counter/windows/CounterApp/CounterHandler.cs}}
```

The app builds the `Core` from it when it launches:

```csharp
// C#: windows/CounterApp/App.xaml.cs
{{#include ../../../examples/counter/windows/CounterApp/App.xaml.cs:on_launched}}
```

`Core` raises `PropertyChanged` for its `View`, which the
[`CounterViewModel`](https://github.com/redbadger/crux/blob/master/examples/counter/windows/CounterApp/CounterViewModel.cs)
mirrors into a property the window binds to.

## Rust shells

A shell written in Rust, like
[Leptos](../part-1/shell/web/leptos.md), uses the core directly and matches on
the `Effect` enum itself. There's no generated `Core` to use there, and no need
for one: in Rust, the match is already as precise as a handler interface.

```admonish success
Run `just dev` in the shell's directory, then open the project in Xcode or
Android Studio (or Visual Studio on Windows), or run `just serve` for the web, and you have a working
counter. Next, let's [give it a capability](./capabilities.md).
```
