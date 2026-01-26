import RivetKitClient

extension ActorError {
    static func clientError(code: String, message: String, metadata: JSONValue? = nil) -> ActorError {
        ActorError(group: "client", code: code, message: message, metadata: metadata)
    }
}
