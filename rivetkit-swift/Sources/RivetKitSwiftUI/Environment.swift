import RivetKitClient
import SwiftUI

public struct RivetKitContext: Sendable {
    public let client: RivetKitClient?
    public let error: ActorError?

    public init(client: RivetKitClient?, error: ActorError?) {
        self.client = client
        self.error = error
    }

    /// Creates a context wrapping an existing client.
    public init(client: RivetKitClient) {
        self.client = client
        self.error = nil
    }
}

/// Singleton actor cache shared across the entire app.
/// This cache is @MainActor-isolated and manages shared actor connections.
@MainActor
private enum ActorCacheHolder {
    static let shared = ActorCache()
}

/// Access the shared actor cache (MainActor-isolated).
@MainActor
public var sharedActorCache: ActorCache {
    ActorCacheHolder.shared
}

private struct RivetKitContextKey: EnvironmentKey {
    // Default context with no client - requires explicit configuration
    static let defaultValue: RivetKitContext = RivetKitContext(
        client: nil,
        error: ActorError.clientError(code: "not_configured", message: "RivetKit not configured. Use .rivetKit(endpoint:) or .rivetKit(client:) to configure.")
    )
}

extension EnvironmentValues {
    var rivetKitContext: RivetKitContext {
        get { self[RivetKitContextKey.self] }
        set { self[RivetKitContextKey.self] = newValue }
    }
}

/// Holder for singleton client instances keyed by endpoint.
/// This ensures that calling .rivetKit(endpoint:) with the same endpoint
/// in different views reuses the same client instance.
@MainActor
private enum ClientHolder {
    private static var clients: [String: RivetKitClient] = [:]

    static func getOrCreate(endpoint: String) throws -> RivetKitClient {
        if let existing = clients[endpoint] {
            return existing
        }
        let client = try RivetKitClient(endpoint: endpoint)
        clients[endpoint] = client
        return client
    }
}

public extension View {
    /// Configures RivetKit with the specified endpoint.
    /// Safe to call in view body - uses a singleton client instance per endpoint.
    ///
    /// Example:
    /// ```swift
    /// @main
    /// struct MyApp: App {
    ///     var body: some Scene {
    ///         WindowGroup {
    ///             ContentView()
    ///                 .rivetKit(endpoint: "http://localhost:6420")
    ///         }
    ///     }
    /// }
    /// ```
    @MainActor
    func rivetKit(endpoint: String) -> some View {
        let context: RivetKitContext
        do {
            let client = try ClientHolder.getOrCreate(endpoint: endpoint)
            context = RivetKitContext(client: client)
        } catch {
            context = RivetKitContext(client: nil, error: ActorError.clientError(code: "config_error", message: error.localizedDescription))
        }
        return environment(\.rivetKitContext, context)
    }

    /// Configures RivetKit with an existing client instance.
    /// The client should be stored as a property (not created inline) to avoid recreation on each render.
    ///
    /// Example:
    /// ```swift
    /// @main
    /// struct MyApp: App {
    ///     private let client = try! RivetKitClient(endpoint: "...")
    ///
    ///     var body: some Scene {
    ///         WindowGroup {
    ///             ContentView()
    ///                 .rivetKit(client: client)
    ///         }
    ///     }
    /// }
    /// ```
    func rivetKit(client: RivetKitClient) -> some View {
        environment(\.rivetKitContext, RivetKitContext(client: client))
    }
}
