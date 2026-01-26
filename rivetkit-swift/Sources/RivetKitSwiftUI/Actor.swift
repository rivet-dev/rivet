import RivetKitClient
import SwiftUI

/// Helper class to track cache lifecycle state for a single @Actor instance.
/// This is @MainActor-isolated and manages the mount/unmount reference counting.
@MainActor
private final class ActorCacheRef: ObservableObject {
    var currentHash: String?
    // Use nonisolated(unsafe) to allow access from deinit
    // This is safe because deinit is the final access and no other code runs concurrently
    nonisolated(unsafe) var unmountHash: String?

    // ActorObservable uses @Observable, so SwiftUI automatically tracks
    // access to its properties without needing Combine forwarding.
    var observable: ActorObservable?

    /// Sets the unmount callback by storing the hash.
    /// The actual unmount will be performed via the shared cache.
    func setUnmount(hash: String) {
        unmountHash = hash
    }

    /// Clears the unmount callback.
    func clearUnmount() {
        unmountHash = nil
    }

    nonisolated deinit {
        // deinit can be called from any thread, so we must dispatch to MainActor
        // The deferred cleanup in ActorCache handles the case where the view
        // remounts quickly (e.g., SwiftUI strict mode)
        if let hash = unmountHash {
            Task { @MainActor in
                sharedActorCache.unmount(hash: hash)
            }
        }
    }
}

@MainActor
@propertyWrapper
public struct Actor: @preconcurrency DynamicProperty {
    @Environment(\.rivetKitContext) private var context
    @StateObject private var cacheRef = ActorCacheRef()

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
    }

    @MainActor
    public var wrappedValue: ActorObservable {
        // Return the cached observable or create a placeholder
        // The placeholder is only used briefly before update() runs
        if let obs = cacheRef.observable {
            return obs
        } else {
            return ActorObservable(options: buildOptions())
        }
    }

    @MainActor
    public var projectedValue: ActorObservable { wrappedValue }

    @MainActor
    public mutating func update() {
        let options = buildOptions()
        let hash = ActorObservable.computeHash(options)

        // Check if hash changed (different actor or first call)
        if hash != cacheRef.currentHash {
            // Unmount previous if any
            if let oldHash = cacheRef.currentHash {
                sharedActorCache.unmount(hash: oldHash)
                cacheRef.clearUnmount()
            }

            // Get or create from cache
            let (observable, newHash) = sharedActorCache.getOrCreate(options: options, context: context)
            cacheRef.observable = observable
            sharedActorCache.mount(hash: newHash)
            cacheRef.setUnmount(hash: newHash)
            cacheRef.currentHash = hash
        } else if let observable = cacheRef.observable {
            // Same hash, just update context/options
            observable.scheduleUpdate(context: context, options: options)
        }
    }

    private func buildOptions() -> ActorOptions {
        ActorOptions(
            name: name,
            key: key,
            params: params,
            createWithInput: createWithInput,
            createInRegion: createInRegion,
            enabled: enabled
        )
    }
}
