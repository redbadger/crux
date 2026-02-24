# iOS

Lets start with the new part, and also typically the shorter part –
implementing the capabilities.

## Capability implementation

This is what Weather's `core.swift` look like

```swift
{{#include ../../../../examples/weather/iOS/Weather/Core.swift:core_base}}
        // ...
    }
}
```

It's slightly more complicated, but broadly the same as the Counter's core.
We've have an extra logger which is not really important for us, and we
also hold on to a `KeyValueStore`, which is the storage for the key-value
implementation.

We've truncated the `processEffect` method, because it's fairly long, but the basic
structure is this:

```swift
    func processEffect(_ request: Request) {
        switch request.effect {
        case .render:
            DispatchQueue.main.async {
                self.view = try! .bincodeDeserialize(input: [UInt8](self.core.view()))
            }
        case .http(let req):
            // ...

        case .keyValue(let keyValue):
            // ...

        case .location(let locationOp):
            // ...
        }
    }
```

We get a Request, and do an exhaustive match on what the requested effect is. In swift
we have tagged unions, so we can also destructure the operation requested.

We can have a look at what the HTTP branch does:

```swift
{{#include ../../../../examples/weather/iOS/Weather/Core.swift:http}}
```

We start a new `Task` to run this job off the main thread, then we use the `async requestHttp()`
call to run the request.

Then it takes the response, serializes it and passes it to `core.resolve`, which
**returns more effect requests**. This is perhaps unexpected, but it's the direct
consequence of the `Command`s async nature. There can easily be a command which
does something along the lines of:

```rust,noplayground
Command::new(|ctx| {
    let http_req = Http::get(url).expect_json<Counter>().build().into_future(ctx);
    let resp = http_req.await; // effect 1

    let counter = resp.map(|result| match result {
        Ok(mut response) => match response.take_body() {
            Some(counter) => {
                Ok(results)
            }
            None => Err(ApiError::ParseError),
        },
        Err(_) => Err(ApiError::NetworkError),
    });

    let _ = KeyValue::set(COUNTER, counter).into_future(ctx).await // effect 2

    // ...

    ctx.send_event(Event::Done);
})
```

Once we resolve the http request at the `.await` point marked "effect 1", this future can
proceed and make a `KeyValue` request at the "effect 2" `.await` point. So on the
shell end, we need to be able to respond appropriately.

What we do is loop through those effect requests (there could easily be multiple requests
at once), go through them and recurse - call `processEffect` again to handle it.

Just for completeness, this is what `requestHttp` looks like:

```swift
{{#include ../../../../examples/weather/iOS/Weather/http.swift}}
```

Not that interesting, it's a wrapper around `URLRequest` and friends which takes and
returns the generated `HttpRequest` and `HttpResponse`, originall defined in Rust by
`crux_http`.

The pattern repeats similarly for key-value store and the location capability.

## User interface and navigation

It's worth looking at how Weather handles the Workflow navigation in Swift UI.

As in the Counter example, the Weather's core has a `@Published var view: ViewModel`
which we can use in the Views.

Here's the root content view:

```swift
{{#include ../../../../examples/weather/iOS/Weather/ContentView.swift:content_view}}
```

Thanks to the declarative nature of SwiftUI, we can show the view we need to,
depending on the workflow, and pass the core down.

We could do this differently - core could stay in the root view and we could pass
an `update` callback in an environment, and just the appropriate section of the
view model to each view, it's up to you how you want to go about it.

Let's look at the HomeView as well, just to complete the picture:

```swift
{{#include ../../../../examples/weather/iOS/Weather/HomeView.swift:home_view}}
```

It simply caters for the possible situations in the view model, draws the
weather cards for each favorite and adds a toolbar with an item, which
when tapped calls `core.update` with the swift equivalent of the `.navigate`
event we saw earlier in the call.

This is quite a simple navigation setup in that it is a static set of screens
we're managing. Sometimes a more dynamic navigation is necessary, but
SwiftUI's `NavigationStack` in recent iOS supports quite complex scenarios in
a declarative fashion using [`NavigationPath`](https://developer.apple.com/documentation/swiftui/navigationpath),
so the general principle of naively projecting the view model into the user
interface broadly works even there.

There isn't much more to it, the rest of the app is rinse and repeat. It is
relatively rare to implement a new capability, so most of the work is in finessing
the user interface. Crux tends to work reasonably well with SwiftUI previews as well
so you can typically avoid the Simulator or device for the inner development loop.

## What's next

Congratulations! You know now all you will likely need to build Crux apps. The
following parts of the book will cover advanced topics, other support platforms,
and internals of Crux, should you be interested in how things work.

Happy building!
