# The Weather App

So far, we've explained the basics on a very simple counter app. So simple in fact, that
it barely demonstrated any of the key features of Crux.

Time to ditch the training wheels and dive into something real. We'll need to demonstrate
a few key concepts. How the Elm architecture works at a larger scale, how we manage
navigation in a multi-screen apps, and the main focus will be on managed effects and capabilities.
To that end, we'll need an app that does enough interesting things, while staying reasonably small.

So we're going to build a Weather app. It is certainly interesting enough - it needs to call an API,
store some data locally, and even uses location APIs to show local weather. That's plenty of effects
for us to play with and see how Crux supports this.

**TODO**: add iOS and Android screenshots

The app works very similarly to a system weather utility: you get multiple screens with basic weather
information and forecast, you get to search for locations and save favourites.

You can look at the [full example code](https://github.com/redbadger/crux/tree/master/examples/weather)
in the Crux Github repo, but we'll walk through the key parts. As before, we're going to start with the core
and once we have it, look at the shells.

Unlike in Part I, we will not build the app step by step, it would be very long and repetitive, we will
instead do more of a code review of the key parts.

Before we dive in though, lets quickly establish some foundations about the app architecture Crux follows,
known most widely as the Elm architecture, based on the language which popularised it.
