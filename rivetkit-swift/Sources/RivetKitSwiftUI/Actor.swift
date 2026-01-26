import RivetKitClient
import SwiftUI

@propertyWrapper
public struct Actor: DynamicProperty {
    @Environment(\.rivetKitContext) private var context
    @StateObject private var observable: ActorObservable

    private let name: String
    private let key: [String]
    private let params: AnyEncodable?
    private let createWithInput: AnyEncodable?
    private let createInRegion: String?
    private let enabled: Bool

    @MainActor
    public init(
        _ name: String,
        key: String,
        params: (any Encodable)? = nil,
        createWithInput: (any Encodable)? = nil,
        createInRegion: String? = nil,
        enabled: Bool = true
    ) {
        self.name = name
        self.key = [key]
        self.params = params.map { AnyEncodable($0) }
        self.createWithInput = createWithInput.map { AnyEncodable($0) }
        self.createInRegion = createInRegion
        self.enabled = enabled
        _observable = StateObject(wrappedValue: ActorObservable(options: ActorOptions(
            name: name,
            key: [key],
            params: params.map { AnyEncodable($0) },
            createWithInput: createWithInput.map { AnyEncodable($0) },
            createInRegion: createInRegion,
            enabled: enabled
        )))
    }

    @MainActor
    public init(
        _ name: String,
        key: [String],
        params: (any Encodable)? = nil,
        createWithInput: (any Encodable)? = nil,
        createInRegion: String? = nil,
        enabled: Bool = true
    ) {
        self.name = name
        self.key = key
        self.params = params.map { AnyEncodable($0) }
        self.createWithInput = createWithInput.map { AnyEncodable($0) }
        self.createInRegion = createInRegion
        self.enabled = enabled
        _observable = StateObject(wrappedValue: ActorObservable(options: ActorOptions(
            name: name,
            key: key,
            params: params.map { AnyEncodable($0) },
            createWithInput: createWithInput.map { AnyEncodable($0) },
            createInRegion: createInRegion,
            enabled: enabled
        )))
    }

    @MainActor
    public var wrappedValue: ActorObservable { observable }
    @MainActor
    public var projectedValue: ActorObservable { observable }

    public mutating func update() {
        let options = ActorOptions(
            name: name,
            key: key,
            params: params,
            createWithInput: createWithInput,
            createInRegion: createInRegion,
            enabled: enabled
        )
        let context = context
        let observable = observable
        Task { @MainActor in
            observable.update(context: context, options: options)
        }
    }
}
