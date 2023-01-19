# Capability APIs

In the previous chapter, we looked at the purpose of Capabilities and using them in Crux apps. In this one, we'll go through building our own. It will be a simple one, but real enough to show the key parts.

We'll extend the Counter example we've built in the [Hello World](hello_world.md) chapter and make it _worse_. Intentionally. We'll add a random delay before we actually update the counter, just to annoy the user. It's a silly example, but it will allow us to demonstrate a few things:

- Random numbers, current time and delay are also side-effects
- To introduce a random delay, we will need to chain two effects behind a single capability call
- The capability can also offer specific time delay API and we can show how capabilities with multiple _operations_ work.

TODO go thorough the implementation
