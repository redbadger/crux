# Motivation

We set out to prove this approach to building apps largely because we've seen
the drawbacks of all the other approaches in real life, and thought "there must
be a better way". The two major available approaches to building the same
application for iOS and Android are:

1. Build a native app for each platform, effectively doing the work twice.
2. Use React Native or Flutter to build the application once[^once] and produce
   native looking and feeling apps which behave nearly identically.

The drawback of the first approach is doing the work twice. In order to build
every feature for iOS and Android at the same time, you need twice the number of
people, either people who happily do Swift and Kotlin (and they are very rare),
or more likely a set of iOS engineers and another set of Android engineers. This
typically leads to forming two separate, platform-focused teams. We have
witnessed situations first-hand, where those teams struggle with the same design
problems, and despite one encountering and solving the problem first, the other
one can learn nothing from their experience (and that's _despite_ long design
discussions).

We think such experiences with the platform native approach are common, and the
reason why people look to React Native and Flutter. The issues with React Native
are two fold

- Only _mostly_ native user interface
- In the case of React Native, the JavaScript ecosystem tooling disaster

React Native effectively takes over, and works hard to insulate the engineer
from the native platform underneath and pretend it doesn't really exist, but of
course, inevitably, it does and the user interface ends up being built in a
combination of 90% JavaScript/TypeScript and 10% Kotlin/Swift. This was still a
major win when React Native was first introduced, because the platform native UI
toolkits were imperative, following a version of MVC architecture, and generally
made it quite difficult to get UI state management right. React on the other
hand is declarative, leaving much less space for errors stemming from the UI
getting into an undefined state. This benefit was clearly recognised by iOS and
Android, and both introduced their own declarative UI toolkit - Swift UI and
Jetpack Compose. Both of them are quite good, matching that particular advantage
of React Native, and leaving only building things once (in theory). But in
exchange, they have to be written in JavaScript (and adjacent tools and
languages).

The main issue with the JavaScript ecosystem is that it's built on sand. The
underlying language is quite loose and has a
[lot of inconsistencies](https://www.destroyallsoftware.com/talks/wat). It came
with no package manager originally, now [it](https://www.npmjs.com/)
[has](https://yarnpkg.com/) [three](https://pnpm.io/). To serve code to the
browser, it gets bundled, and the list of bundlers is too long to include here.
[Webpack](https://webpack.js.org/), the most popular one, is famously difficult
to configure. JavaScript was built as a dynamic language which leads to a lot of
basic human errors, which are made while writing the code, only being discovered
when running the code. Static type systems aim to solve that problem and
[TypeScript](https://www.typescriptlang.org/) adds this onto JavaScript, but the
types only go so far (until they hit an `any` type, or dependencies with no type
definitions), and they disappear at runtime.

In short, upgrading JavaScript to something modern takes a lot of tooling.
Getting all this tooling set up and ready to build things is an all day job, and
so more tooling, like [Next.js](https://nextjs.org/) has popped up providing
this configuration in a box, batteries included. Perhaps the final admission of
this problem is the recent [Biome](https://biomejs.dev/blog/annoucing-biome/)
toolchain (formerly the Rome project), attempting to bring all the various tools
under one roof (and Biome itself is built in Rust...).

It's no wonder that even a working setup of all the tooling has sharp edges, and
cannot afford to be nearly as strict as tooling designed with strictness in
mind, such as Rust's. The heart of the problem is that computers are strict and
precise instruments, and humans are sloppy creatures. With enough humans (more
than 10, being generous) and no additional help, the resulting code will be
sloppy, full of unhandled edge cases, undefined behaviour being relied on,
circular dependencies preventing testing in isolation, etc. (and yes, these are
not hypotheticals).

Contrast that with Rust, which is as strict as it gets, and generally backs up
the claim that if it compiles it will work (and if you struggle to get it past
the compiler, it's probably a bad idea). The tooling and package management is
built in with `cargo`. There are fewer decisions to make when setting up a Rust
project.

In short, we think the JS ecosystem has jumped the shark, the "complexity
toothpaste" is out of the tube, and it's time to stop. But there's no real
viable alternative. Crux is our attempt to provide one.

---

[^once]:
    In reality it's more like 1.4x effort build the same app for two platforms.
