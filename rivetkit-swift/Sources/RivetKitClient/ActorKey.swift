import Foundation

enum ActorKeyCodec {
    private static let emptyKeyMarker = "/"
    private static let separator = "/"

    static func serialize(_ key: [String]) -> String {
        if key.isEmpty {
            return emptyKeyMarker
        }

        let escapedParts = key.map { part -> String in
            if part.isEmpty {
                return "\\0"
            }
            var escaped = part.replacingOccurrences(of: "\\", with: "\\\\")
            escaped = escaped.replacingOccurrences(of: "/", with: "\\\(separator)")
            return escaped
        }

        return escapedParts.joined(separator: separator)
    }
}
