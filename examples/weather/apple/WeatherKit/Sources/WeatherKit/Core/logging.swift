import os

enum Log {
    private static let subsystem = "com.crux.examples.weather"

    static let core = Logger(subsystem: subsystem, category: "core")
    static let http = Logger(subsystem: subsystem, category: "http")
    static let time = Logger(subsystem: subsystem, category: "time")
    static let secret = Logger(subsystem: subsystem, category: "secret")
    static let kv = Logger(subsystem: subsystem, category: "kv")
    static let location = Logger(subsystem: subsystem, category: "location")
}
