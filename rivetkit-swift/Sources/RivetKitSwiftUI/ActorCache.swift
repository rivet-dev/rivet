import Foundation
import RivetKitClient

/// A cache that manages shared actor connections.
/// Multiple `@Actor` property wrappers with the same options share a single connection.
@MainActor
public final class ActorCache {
    /// A cached actor entry with reference counting.
    final class CacheEntry {
        let observable: ActorObservable
        var refCount: Int = 0
        var cleanupTask: Task<Void, Never>?

        init(observable: ActorObservable) {
            self.observable = observable
        }
    }

    private var cache: [String: CacheEntry] = [:]

    public init() {}

    /// Gets or creates a cached actor observable for the given options.
    /// Returns the observable and the hash for lifecycle management.
    func getOrCreate(
        options: ActorOptions,
        context: RivetKitContext
    ) -> (observable: ActorObservable, hash: String) {
        let hash = ActorObservable.computeHash(options)

        if let existing = cache[hash] {
            // Stage the context for potential use by runStagedUpdateIfNeeded.
            // If there's already an active connection, the staged update will be skipped.
            existing.observable.stage(context: context, options: options)
            return (existing.observable, hash)
        }

        let observable = ActorObservable(options: options)
        observable.stage(context: context, options: options)

        let entry = CacheEntry(observable: observable)
        cache[hash] = entry

        return (observable, hash)
    }

    /// Mounts an actor observable, incrementing its ref count.
    /// Should be called when a view starts using the actor.
    func mount(hash: String) {
        guard let entry = cache[hash] else { return }

        // Cancel pending cleanup
        entry.cleanupTask?.cancel()
        entry.cleanupTask = nil

        // Increment ref count
        entry.refCount += 1

        // Run staged update if needed. The observable's runStagedUpdateIfNeeded
        // will skip if there's already an active connection.
        entry.observable.runStagedUpdateIfNeeded()
    }

    /// Unmounts an actor observable, decrementing its ref count.
    /// Should be called when a view stops using the actor.
    /// When ref count reaches 0, the connection is disposed after a short delay.
    func unmount(hash: String) {
        guard let entry = cache[hash] else { return }

        entry.refCount -= 1

        if entry.refCount == 0 {
            // Deferred cleanup prevents needless reconnection when:
            // - SwiftUI preview's mount/unmount cycle
            // - View re-renders cause property wrapper recreation
            // - @StateObject not preserving state in property wrapper
            //
            // Use a longer delay (5 seconds) to handle SwiftUI's async view lifecycle.
            // The delay is cancelled immediately if a new mount happens.
            entry.cleanupTask = Task { @MainActor [weak self] in
                // Wait for any pending mounts to happen
                try? await Task.sleep(for: .seconds(5))

                guard let self else { return }
                guard let current = self.cache[hash], current.refCount == 0 else { return }

                current.observable.dispose()
                self.cache.removeValue(forKey: hash)
            }
        }
    }
}
