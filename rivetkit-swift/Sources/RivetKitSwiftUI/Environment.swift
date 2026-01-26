import RivetKitClient
import SwiftUI

public struct RivetKitContext: Sendable {
    public let client: RivetKitClient?
    public let error: ActorError?

    public init(client: RivetKitClient?, error: ActorError?) {
        self.client = client
        self.error = error
    }
}

private struct RivetKitContextKey: EnvironmentKey {
    static let defaultValue: RivetKitContext = RivetKitContext.makeDefault()
}

extension EnvironmentValues {
    var rivetKitContext: RivetKitContext {
        get { self[RivetKitContextKey.self] }
        set { self[RivetKitContextKey.self] = newValue }
    }
}

private final class RivetKitContextBox: ObservableObject {
    let context: RivetKitContext

    init(context: RivetKitContext) {
        self.context = context
    }

    deinit {
        if let client = context.client {
            Task {
                await client.dispose()
            }
        }
    }
}

private struct RivetKitProvider: ViewModifier {
    @StateObject private var box: RivetKitContextBox

    init(context: RivetKitContext) {
        _box = StateObject(wrappedValue: RivetKitContextBox(context: context))
    }

    func body(content: Content) -> some View {
        content.environment(\.rivetKitContext, box.context)
    }
}

public extension View {
    func rivetKit(_ endpoint: String) -> some View {
        modifier(RivetKitProvider(context: RivetKitContext.fromEndpoint(endpoint)))
    }

    func rivetKit(_ config: ClientConfig) -> some View {
        modifier(RivetKitProvider(context: RivetKitContext.fromConfig(config)))
    }
}

extension RivetKitContext {
    static func fromEndpoint(_ endpoint: String) -> RivetKitContext {
        do {
            let config = try ClientConfig(endpoint: endpoint)
            return fromConfig(config)
        } catch {
            return RivetKitContext(client: nil, error: ActorError.clientError(code: "config_error", message: error.localizedDescription))
        }
    }

    static func fromConfig(_ config: ClientConfig) -> RivetKitContext {
        let client = RivetKitClient(config: config)
        return RivetKitContext(client: client, error: nil)
    }

    static func makeDefault() -> RivetKitContext {
        do {
            let client = try RivetKitClient()
            return RivetKitContext(client: client, error: nil)
        } catch {
            return RivetKitContext(client: nil, error: ActorError.clientError(code: "config_error", message: error.localizedDescription))
        }
    }
}
