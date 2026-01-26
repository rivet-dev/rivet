import RivetKitClient
import SwiftUI

public extension View {
    /// Subscribes to an actor event with no arguments.
    func onActorEvent(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping () -> Void
    ) -> some View {
        modifier(ActorEventModifierVoid(actor: actor, event: event, handler: perform))
    }

    func onActorEvent<A: Decodable & Sendable>(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A) -> Void
    ) -> some View {
        modifier(ActorEventModifier<A>(actor: actor, event: event, handler: perform))
    }

    func onActorEvent<A: Decodable & Sendable, B: Decodable & Sendable>(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A, B) -> Void
    ) -> some View {
        modifier(ActorEventModifier2<A, B>(actor: actor, event: event, handler: perform))
    }

    func onActorEvent<A: Decodable & Sendable, B: Decodable & Sendable, C: Decodable & Sendable>(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A, B, C) -> Void
    ) -> some View {
        modifier(ActorEventModifier3<A, B, C>(actor: actor, event: event, handler: perform))
    }

    func onActorEvent<A: Decodable & Sendable, B: Decodable & Sendable, C: Decodable & Sendable, D: Decodable & Sendable>(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A, B, C, D) -> Void
    ) -> some View {
        modifier(ActorEventModifier4<A, B, C, D>(actor: actor, event: event, handler: perform))
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
        modifier(ActorEventModifier5<A, B, C, D, E>(actor: actor, event: event, handler: perform))
    }

    /// Raw JSON event arguments. Use this when you need more than 5 positional arguments.
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

private struct ActorEventModifierVoid: ViewModifier {
    let actor: ActorObservable
    let event: String
    let handler: () -> Void

    func body(content: Content) -> some View {
        content
            .task(id: ActorEventKey(connectionId: actor.connectionToken(), event: event)) {
                for await _ in actor.events(event, as: Void.self) {
                    handler()
                }
            }
    }
}

private struct ActorEventModifier<T: Decodable & Sendable>: ViewModifier {
    let actor: ActorObservable
    let event: String
    let handler: (T) -> Void

    func body(content: Content) -> some View {
        content
            .task(id: ActorEventKey(connectionId: actor.connectionToken(), event: event)) {
                for await value in actor.events(event, as: T.self) {
                    handler(value)
                }
            }
    }
}

private struct ActorEventModifier2<A: Decodable & Sendable, B: Decodable & Sendable>: ViewModifier {
    let actor: ActorObservable
    let event: String
    let handler: (A, B) -> Void

    func body(content: Content) -> some View {
        content
            .task(id: ActorEventKey(connectionId: actor.connectionToken(), event: event)) {
                for await tuple in actor.events(event, as: (A, B).self) {
                    handler(tuple.0, tuple.1)
                }
            }
    }
}

private struct ActorEventModifier3<A: Decodable & Sendable, B: Decodable & Sendable, C: Decodable & Sendable>: ViewModifier {
    let actor: ActorObservable
    let event: String
    let handler: (A, B, C) -> Void

    func body(content: Content) -> some View {
        content
            .task(id: ActorEventKey(connectionId: actor.connectionToken(), event: event)) {
                for await tuple in actor.events(event, as: (A, B, C).self) {
                    handler(tuple.0, tuple.1, tuple.2)
                }
            }
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

    func body(content: Content) -> some View {
        content
            .task(id: ActorEventKey(connectionId: actor.connectionToken(), event: event)) {
                for await tuple in actor.events(event, as: (A, B, C, D).self) {
                    handler(tuple.0, tuple.1, tuple.2, tuple.3)
                }
            }
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

    func body(content: Content) -> some View {
        content
            .task(id: ActorEventKey(connectionId: actor.connectionToken(), event: event)) {
                for await tuple in actor.events(event, as: (A, B, C, D, E).self) {
                    handler(tuple.0, tuple.1, tuple.2, tuple.3, tuple.4)
                }
            }
    }
}

private struct ActorEventModifierRaw: ViewModifier {
    let actor: ActorObservable
    let event: String
    let handler: ([JSONValue]) -> Void

    func body(content: Content) -> some View {
        content
            .task(id: ActorEventKey(connectionId: actor.connectionToken(), event: event)) {
                for await args in actor.events(event) {
                    handler(args)
                }
            }
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
