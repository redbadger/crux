import App
import Observation

/// Lightweight event dispatcher injected via `@Environment(CoreUpdater.self)`.
/// Previews use `CoreUpdater.forPreview()` for a no-op.
@Observable
@MainActor
public final class CoreUpdater {
    private let handler: (Event) -> Void

    public init(_ handler: @escaping (Event) -> Void) {
        self.handler = handler
    }

    public func callAsFunction(_ event: Event) {
        handler(event)
    }

    #if DEBUG
        public static func forPreview() -> CoreUpdater {
            CoreUpdater { _ in }
        }
    #endif
}
