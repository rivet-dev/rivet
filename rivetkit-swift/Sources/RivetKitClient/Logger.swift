import Foundation
import os

/// Internal logging for RivetKit using OSLog.
/// Uses the subsystem "dev.rivet.client" with category-specific loggers.
enum RivetLogger {
    private static let subsystem = "dev.rivet.client"

    static let client = Logger(subsystem: subsystem, category: "client")
    static let connection = Logger(subsystem: subsystem, category: "connection")
    static let handle = Logger(subsystem: subsystem, category: "handle")
    static let manager = Logger(subsystem: subsystem, category: "manager")
}
