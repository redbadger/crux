# Motivation

We set out to prove this approach to building apps largely because we've seen the drawbacks of all the other approaches in real life, and thought "there must be a better way". The two major available approaches to building the same application for iOS and Android are:

1. Build a native app for each platform, effectively doing the work twice.
2. Use React Native or Flutter to build the application once[^once] and produce native looking and feeling apps which behave nearly identically.

The drawback of the first approach is doing the work twice. In order to build every feature for iOS and Android at the same time, you need twice the number of people, either people who happily do Swift and Kotlin (and they are very rare), or more likely a set of iOS engineers and another set of Android engineers. This typically leads to forming two separate, platform-focused teams. We have witnessed first-hand situations where those teams struggle with the same design problems, and despite one encountering and solving the problem first, the other one can learn nothing from their experience despite long design discussions.

We think such experience with the platform native approach are common, and the reason why people look at React Native and Flutter. The issues with React Native are two fold

* Mostly-native user interface
* The JavaScript ecosystem tooling disaster

React Native effectively takes over, and works hard to insulate the engineer from the native platform underneath and pretend it doesn't really exist, but of course, inevitably, it does and the user interface ends up being built in a combination of 90% JavaScript/TypeScript and 10% Kotlin/Swift UI. This was a major win when React Native was first introduced, because the platform native UI toolkits were imperative, following an MVC architecture or similar, and generally made it quite difficult to get state management right. React on the other hand is declarative, leaving much less space for error from the UI getting into an undefined state. This benefit was clearly recognised by Apple and Google, and both introduced their own declarative UI toolkit - Swifth UI and Jetpack Compose. Both of them are quite good, removing that particular advantage of React Native, and leaving only building things once (in theory).

The main issue with the JavaScript ecosystem is that it's built on sand. The underlying language is quite loose and has a [lot of incosistencies](https://www.destroyallsoftware.com/talks/wat). It came with no package manager originally, now [it](https://www.npmjs.com/) [has](https://yarnpkg.com/) [three](https://pnpm.io/). To serve code to the browser, it gets bundled, and the list of bundlers is too long to include here. [Webpack](https://webpack.js.org/), the most popular one is famously difficult to configure. It was built as a dynamic language which leads to a lot of basic human errors, which are made while writing the code, only being discovered when running the code. Static type systems aim to solve that problem and [TypeScript](https://www.typescriptlang.org/) adds this onto JavaScript, but the types only go so far (until they hit an `any` type, or dependencies with no type definitions), and they disappear at runtime. Getting all this tooling set up and ready to build things is an all day job, and so more tooling, like [Next.js](https://nextjs.org/) has popped up providing this configuration in a box, batteries included. Perhaps the final admission of this problem is the recent [Rome tools](https://rome.tools/) project, attempting to bring all the various tools under one roof.

It's no wonder that even a working setup of all the tooling has sharp edges, and cannot afford to be nearly as strict as well designed, strict tooling, such as Rust's. The problem is that computers are strict and precise instruments, and humans are sloppy creatures. With enough humans (more than 10, being generous) and no additional help, the resulting code will be sloppy, full of unhandled edge cases, undefined behaviour being relied on, circular dependencies preventing testing in isolation, etc. (yes, this is a real-world example).

Contrast that with Rust, which is as strict as it gets, and generally backs up the claim that if it compiles it will work (and if you struggle to get it past the compiler, it's probably a bad idea). The tooling and package management is built in with `cargo`. There are much fewer decisions to make when setting up a Rust project.

In short, we think the JS ecosystem has jumped the shark, the complexity toothpaste is out of the tube, and it's time to stop. But there's no real viable alternative. RMM is our attempt to provide one.

---
[^once]: In reality it's more like 1.4x effort build the same app for two platforms.
