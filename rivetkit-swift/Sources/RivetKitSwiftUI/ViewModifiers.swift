import RivetKitClient
import SwiftUI

public extension View {
    func onActorEvent(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping () -> Void
    ) -> some View {
        modifier(ActorEventModifier0(actor: actor, event: event, handler: perform))
    }

    func onActorEvent<A: Decodable>(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A) -> Void
    ) -> some View {
        modifier(ActorEventModifier1(actor: actor, event: event, handler: perform))
    }

    func onActorEvent<A: Decodable, B: Decodable>(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A, B) -> Void
    ) -> some View {
        modifier(ActorEventModifier2(actor: actor, event: event, handler: perform))
    }

    func onActorEvent<A: Decodable, B: Decodable, C: Decodable>(
        _ actor: ActorObservable,
        _ event: String,
        perform: @escaping (A, B, C) -> Void
    ) -> some View {
        modifier(ActorEventModifier3(actor: actor, event: event, handler: perform))
    }

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
    let connectionId: UUID?
    let event: String
}

private struct ActorEventModifier0: ViewModifier {
    @ObservedObject var actor: ActorObservable
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
        let unsubscribe = await connection.on(event) { args in
            Task { @MainActor in
                let decoder = EventDecoder(actor: actor, eventName: event)
                guard decoder.decodeZero(args: args) else { return }
                handler()
            }
        }
        self.unsubscribe = unsubscribe
    }
}

private struct ActorEventModifier1<A: Decodable>: ViewModifier {
    @ObservedObject var actor: ActorObservable
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
        let unsubscribe = await connection.on(event) { args in
            Task { @MainActor in
                let decoder = EventDecoder(actor: actor, eventName: event)
                guard let value = decoder.decodeSingle(args: args, as: A.self) else { return }
                handler(value)
            }
        }
        self.unsubscribe = unsubscribe
    }
}

private struct ActorEventModifier2<A: Decodable, B: Decodable>: ViewModifier {
    @ObservedObject var actor: ActorObservable
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
        let unsubscribe = await connection.on(event) { args in
            Task { @MainActor in
                let decoder = EventDecoder(actor: actor, eventName: event)
                guard let value = decoder.decodePair(args: args, as: (A, B).self) else { return }
                handler(value.0, value.1)
            }
        }
        self.unsubscribe = unsubscribe
    }
}

private struct ActorEventModifier3<A: Decodable, B: Decodable, C: Decodable>: ViewModifier {
    @ObservedObject var actor: ActorObservable
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
        let unsubscribe = await connection.on(event) { args in
            Task { @MainActor in
                let decoder = EventDecoder(actor: actor, eventName: event)
                guard let value = decoder.decodeTriple(args: args, as: (A, B, C).self) else { return }
                handler(value.0, value.1, value.2)
            }
        }
        self.unsubscribe = unsubscribe
    }
}

private struct ActorEventModifierRaw: ViewModifier {
    @ObservedObject var actor: ActorObservable
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
    @ObservedObject var actor: ActorObservable
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
