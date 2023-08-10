# Apple Tap to Pay demo

As an example of a simple app with a navigation structure. This is at best a clickable prototype, but built such that a real capability could be plugged in
to support the payment processing.

In place of real payment processing, we show a pretend tap to pay screen and use a timer capability to behave as if the payment is being processed in the background.

## Navigation approach

The [core's `ViewModel`](./shared/src/app.rs#L29) is a struct, where one of the fields is `screen` of type
`Screen`. Screen is an enum of the various screens and their associated data. At the moment the only screen there is `Payment`.

The main point of the demo is to show how the SwiftUI app uses this view model
to decide what to present using SwiftUI's `NavigationStack` and some bindings
to help convert the view model into the right types expected by the components (e.g. bools deciding whether modal sheets should be presented).
