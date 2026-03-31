import SwiftUI

#if canImport(UIKit)
import UIKit
let platformBackground = UIColor.systemBackground
let platformGroupedBackground = UIColor.systemGroupedBackground
let platformSecondaryBackground = UIColor.secondarySystemBackground
#else
import AppKit
let platformBackground = NSColor.windowBackgroundColor
let platformGroupedBackground = NSColor.controlBackgroundColor
let platformSecondaryBackground = NSColor.controlBackgroundColor
#endif
