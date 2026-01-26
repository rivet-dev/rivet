import RivetKitClient
import SwiftUI

public extension View {
    /// Subscribes to an actor event with no arguments.
    /// Use the typed overloads for 1-3 arguments to decode positional values.
    func onActorEvent(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping () -> Void
    ) -> some View {
        modifier(ActorEventModifier0(actor: actor, event: event, handler: perform))
    }

    func onActorEvent<A: Decodable & Sendable>(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A) -> Void
    ) -> some View {
        modifier(ActorEventModifier1(actor: actor, event: event, handler: perform))
    }

    func onActorEvent<A: Decodable & Sendable, B: Decodable & Sendable>(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A, B) -> Void
    ) -> some View {
        modifier(ActorEventModifier2(actor: actor, event: event, handler: perform))
    }

    func onActorEvent<A: Decodable & Sendable, B: Decodable & Sendable, C: Decodable & Sendable>(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A, B, C) -> Void
    ) -> some View {
        modifier(ActorEventModifier3(actor: actor, event: event, handler: perform))
    }

    func onActorEvent<A: Decodable & Sendable, B: Decodable & Sendable, C: Decodable & Sendable, D: Decodable & Sendable>(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A, B, C, D) -> Void
    ) -> some View {
        modifier(ActorEventModifier4(actor: actor, event: event, handler: perform))
    }

    func onActorEvent<
        A: Decodable & Sendable,
        B: Decodable & Sendable,
        C: Decodable & Sendable,
        D: Decodable & Sendable,
        E: Decodable & Sendable
    >(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A, B, C, D, E) -> Void
    ) -> some View {
        modifier(ActorEventModifier5(actor: actor, event: event, handler: perform))
    }

    /// Raw JSON event arguments. Use this when you need more than 5 positional arguments.
    @available(*, deprecated, message: "use typed event overloads instead of raw JSON values")
    func onActorEvent(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping ([JSONValue]) -> Void
    ) -> some View {
        modifier(ActorEventModifierRaw(actor: actor, event: event, handler: perform))
    }

    func onActorError(
        _ actor: ActorObservable,
        perform: @escaping (ActorError) -> Void
    ) -> some View {
        modifier(ActorErrorModifier(actor: actor, handler: perform))
    }
}

private struct ActorEventKey: Hashable {
    let connectionId: ObjectIdentifier?
    let event: String
}

private struct ActorEventModifier0: ViewModifier {
    let actor: ActorObservable
    let event: String
    let handler: () -> Void
    @State private var unsubscribe: EventUnsubscribe?

    func body(content: Content) -> some View {
        content
            .task(id: ActorEventKey(connectionId: actor.connectionToken(), event: event)) {
                await subscribe()
            }
            .onDisappear {
                let unsubscribe = self.unsubscribe
                self.unsubscribe = nil
                if let unsubscribe {
                    Task { await unsubscribe() }
                }
            }
    }

    private func subscribe() async {
        if let unsubscribe {
            await unsubscribe()
            self.unsubscribe = nil
        }
        guard let connection = actor.connection else { return }
        let unsubscribe = await connection.on(event) {
            Task { @MainActor in
                handler()
            }
        }
        self.unsubscribe = unsubscribe
    }
}

private struct ActorEventModifier1<A: Decodable & Sendable>: ViewModifier {
    let actor: ActorObservable
    let event: String
    let handler: (A) -> Void
    @State private var unsubscribe: EventUnsubscribe?

    func body(content: Content) -> some View {
        content
            .task(id: ActorEventKey(connectionId: actor.connectionToken(), event: event)) {
                await subscribe()
            }
            .onDisappear {
                let unsubscribe = self.unsubscribe
                self.unsubscribe = nil
                if let unsubscribe {
                    Task { await unsubscribe() }
                }
            }
    }

    private func subscribe() async {
        if let unsubscribe {
            await unsubscribe()
            self.unsubscribe = nil
        }
        guard let connection = actor.connection else { return }
        let unsubscribe = await connection.on(event) { (value: A) in
            Task { @MainActor in
                handler(value)
            }
        }
        self.unsubscribe = unsubscribe
    }
}

private struct ActorEventModifier2<A: Decodable & Sendable, B: Decodable & Sendable>: ViewModifier {
    let actor: ActorObservable
    let event: String
    let handler: (A, B) -> Void
    @State private var unsubscribe: EventUnsubscribe?

    func body(content: Content) -> some View {
        content
            .task(id: ActorEventKey(connectionId: actor.connectionToken(), event: event)) {
                await subscribe()
            }
            .onDisappear {
                let unsubscribe = self.unsubscribe
                self.unsubscribe = nil
                if let unsubscribe {
                    Task { await unsubscribe() }
                }
            }
    }

    private func subscribe() async {
        if let unsubscribe {
            await unsubscribe()
            self.unsubscribe = nil
        }
        guard let connection = actor.connection else { return }
        let unsubscribe = await connection.on(event) { (first: A, second: B) in
            Task { @MainActor in
                handler(first, second)
            }
        }
        self.unsubscribe = unsubscribe
    }
}

private struct ActorEventModifier3<A: Decodable & Sendable, B: Decodable & Sendable, C: Decodable & Sendable>: ViewModifier {
    let actor: ActorObservable
    let event: String
    let handler: (A, B, C) -> Void
    @State private var unsubscribe: EventUnsubscribe?

    func body(content: Content) -> some View {
        content
            .task(id: ActorEventKey(connectionId: actor.connectionToken(), event: event)) {
                await subscribe()
            }
            .onDisappear {
                let unsubscribe = self.unsubscribe
                self.unsubscribe = nil
                if let unsubscribe {
                    Task { await unsubscribe() }
                }
            }
    }

    private func subscribe() async {
        if let unsubscribe {
            await unsubscribe()
            self.unsubscribe = nil
        }
        guard let connection = actor.connection else { return }
        let unsubscribe = await connection.on(event) { (first: A, second: B, third: C) in
            Task { @MainActor in
                handler(first, second, third)
            }
        }
        self.unsubscribe = unsubscribe
    }
}

private struct ActorEventModifier4<
    A: Decodable & Sendable,
    B: Decodable & Sendable,
    C: Decodable & Sendable,
    D: Decodable & Sendable
>: ViewModifier {
    let actor: ActorObservable
    let event: String
    let handler: (A, B, C, D) -> Void
    @State private var unsubscribe: EventUnsubscribe?

    func body(content: Content) -> some View {
        content
            .task(id: ActorEventKey(connectionId: actor.connectionToken(), event: event)) {
                await subscribe()
            }
            .onDisappear {
                let unsubscribe = self.unsubscribe
                self.unsubscribe = nil
                if let unsubscribe {
                    Task { await unsubscribe() }
                }
            }
    }

    private func subscribe() async {
        if let unsubscribe {
            await unsubscribe()
            self.unsubscribe = nil
        }
        guard let connection = actor.connection else { return }
        let unsubscribe = await connection.on(event) { (first: A, second: B, third: C, fourth: D) in
            Task { @MainActor in
                handler(first, second, third, fourth)
            }
        }
        self.unsubscribe = unsubscribe
    }
}

private struct ActorEventModifier5<
    A: Decodable & Sendable,
    B: Decodable & Sendable,
    C: Decodable & Sendable,
    D: Decodable & Sendable,
    E: Decodable & Sendable
>: ViewModifier {
    let actor: ActorObservable
    let event: String
    let handler: (A, B, C, D, E) -> Void
    @State private var unsubscribe: EventUnsubscribe?

    func body(content: Content) -> some View {
        content
            .task(id: ActorEventKey(connectionId: actor.connectionToken(), event: event)) {
                await subscribe()
            }
            .onDisappear {
                let unsubscribe = self.unsubscribe
                self.unsubscribe = nil
                if let unsubscribe {
                    Task { await unsubscribe() }
                }
            }
    }

    private func subscribe() async {
        if let unsubscribe {
            await unsubscribe()
            self.unsubscribe = nil
        }
        guard let connection = actor.connection else { return }
        let unsubscribe = await connection.on(event) { (first: A, second: B, third: C, fourth: D, fifth: E) in
            Task { @MainActor in
                handler(first, second, third, fourth, fifth)
            }
        }
        self.unsubscribe = unsubscribe
    }
}

@available(*, deprecated, message: "use typed event overloads instead of raw JSON values")
private struct ActorEventModifierRaw: ViewModifier {
    let actor: ActorObservable
    let event: String
    let handler: ([JSONValue]) -> Void
    @State private var unsubscribe: EventUnsubscribe?

    func body(content: Content) -> some View {
        content
            .task(id: ActorEventKey(connectionId: actor.connectionToken(), event: event)) {
                await subscribe()
            }
            .onDisappear {
                let unsubscribe = self.unsubscribe
                self.unsubscribe = nil
                if let unsubscribe {
                    Task { await unsubscribe() }
                }
            }
    }

    private func subscribe() async {
        if let unsubscribe {
            await unsubscribe()
            self.unsubscribe = nil
        }
        guard let connection = actor.connection else { return }
        let unsubscribe = await connection.on(event) { args in
            Task { @MainActor in
                handler(args)
            }
        }
        self.unsubscribe = unsubscribe
    }
}

private struct ActorErrorModifier: ViewModifier {
    let actor: ActorObservable
    let handler: (ActorError) -> Void
    @State private var handlerId: UUID?

    func body(content: Content) -> some View {
        content
            .onAppear {
                if handlerId == nil {
                    handlerId = actor.addErrorHandler(handler)
                }
            }
            .onDisappear {
                if let handlerId {
                    actor.removeErrorHandler(id: handlerId)
                    self.handlerId = nil
                }
            }
    }
}
