import Foundation

public struct ClientConfig: Sendable {
    public var endpoint: String
    public var token: String?
    public var namespace: String
    public var runnerName: String
    public var headers: [String: String]
    public var disableMetadataLookup: Bool

    public init(
        endpoint: String,
        token: String? = nil,
        namespace: String? = nil,
        runnerName: String? = nil,
        headers: [String: String] = [:],
        disableMetadataLookup: Bool = false
    ) throws {
        let env = ProcessInfo.processInfo.environment
        let tokenValue = token ?? env["RIVET_TOKEN"]
        let namespaceValue = namespace ?? env["RIVET_NAMESPACE"]
        let runnerValue = runnerName ?? env["RIVET_RUNNER"] ?? "default"

        let parsed = try EndpointParser.parse(
            endpoint: endpoint,
            namespace: namespaceValue,
            token: tokenValue
        )

        self.endpoint = parsed.endpoint
        self.token = parsed.token ?? tokenValue
        self.namespace = parsed.namespace ?? namespaceValue ?? "default"
        self.runnerName = runnerValue
        self.headers = headers
        self.disableMetadataLookup = disableMetadataLookup
    }
}

struct ParsedEndpoint: Sendable {
    let endpoint: String
    let namespace: String?
    let token: String?
}

enum EndpointParser {
    static func parse(endpoint: String, namespace: String?, token: String?) throws -> ParsedEndpoint {
        guard var components = URLComponents(string: endpoint) else {
            throw ConfigurationError("invalid URL: \(endpoint)")
        }

        if let query = components.percentEncodedQuery, !query.isEmpty {
            throw ConfigurationError("endpoint cannot contain a query string")
        }

        if let fragment = components.fragment, !fragment.isEmpty {
            throw ConfigurationError("endpoint cannot contain a fragment")
        }

        let urlNamespace = components.user?.removingPercentEncoding
        let urlToken = components.password?.removingPercentEncoding

        if urlToken != nil && urlNamespace == nil {
            throw ConfigurationError("endpoint cannot have a token without a namespace")
        }

        if urlNamespace != nil && namespace != nil {
            throw ConfigurationError("cannot specify namespace both in endpoint URL and as a separate config option")
        }

        if urlToken != nil && token != nil {
            throw ConfigurationError("cannot specify token both in endpoint URL and as a separate config option")
        }

        components.user = nil
        components.password = nil

        guard let cleaned = components.url?.absoluteString else {
            throw ConfigurationError("invalid URL: \(endpoint)")
        }

        return ParsedEndpoint(endpoint: cleaned, namespace: urlNamespace, token: urlToken)
    }
}
