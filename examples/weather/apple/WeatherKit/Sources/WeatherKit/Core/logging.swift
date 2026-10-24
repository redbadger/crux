import os

nonisolated enum Log {
    private static let subsystem = "com.crux.examples.weather"

    static let core = Logger(subsystem: subsystem, category: "core")
    static let secret = Logger(subsystem: subsystem, category: "secret")
    static let location = Logger(subsystem: subsystem, category: "location")
}
