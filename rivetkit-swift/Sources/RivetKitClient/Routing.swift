import Foundation

enum Routing {
    static let pathConnect = "/connect"
    static let pathWebSocketPrefix = "/websocket/"

    static let headerConnParams = "x-rivet-conn-params"
    static let headerEncoding = "x-rivet-encoding"
    static let headerRivetToken = "x-rivet-token"

    static let wsProtocolStandard = "rivet"
    static let wsProtocolEncoding = "rivet_encoding."
    static let wsProtocolConnParams = "rivet_conn_params."
}

enum WebSocketCloseReasonParser {
    static func parse(_ reason: String) -> (group: String, code: String, rayId: String?)? {
        let parts = reason.split(separator: "#", maxSplits: 1, omittingEmptySubsequences: false)
        let main = parts.first.map(String.init) ?? ""
        let rayId = parts.count > 1 ? String(parts[1]) : nil
        let groupCode = main.split(separator: ".", maxSplits: 1, omittingEmptySubsequences: false)
        guard groupCode.count == 2 else {
            return nil
        }
        let group = String(groupCode[0])
        let code = String(groupCode[1])
        if group.isEmpty || code.isEmpty {
            return nil
        }
        return (group, code, rayId)
    }
}
