import SwiftUI

extension Color {
    #if canImport(UIKit)
    static let systemBackground = Color(uiColor: .systemBackground)
    static let systemGroupedBackground = Color(uiColor: .systemGroupedBackground)
    static let secondarySystemBackground = Color(uiColor: .secondarySystemBackground)
    #else
    static let systemBackground = Color(nsColor: .windowBackgroundColor)
    static let systemGroupedBackground = Color(nsColor: .controlBackgroundColor)
    static let secondarySystemBackground = Color(nsColor: .controlBackgroundColor)
    #endif
}
