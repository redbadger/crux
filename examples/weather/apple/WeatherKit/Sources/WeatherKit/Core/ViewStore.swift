import App
import Observation

/// Holds the latest view model for SwiftUI, updated from the `onView` callback
/// the generated `Core` calls after every `Render`.
///
/// The `Core` cannot be `@Observable` itself: it is generated with an
/// `@available(macOS 10.15, iOS 13.0, …)` annotation, and `@Observable`
/// requires macOS 14 / iOS 17. Wrapping it in a tiny observable box costs one
/// assignment per render and keeps `@Environment(ViewStore.self)` working in
/// views and previews alike.
@Observable
@MainActor
public final class ViewStore {
    public var view: ViewModel

    public init(view: ViewModel = .loading) {
        self.view = view
    }
}
