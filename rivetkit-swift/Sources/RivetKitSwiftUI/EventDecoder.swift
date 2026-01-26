import Foundation
import RivetKitClient

@MainActor
struct EventDecoder {
    let actor: ActorObservable
    let eventName: String

    func decodeSingle<T: Decodable>(args: [JSONValue], as _: T.Type) -> T? {
        guard args.count == 1 else {
            actor.reportDecodeError(eventName: eventName, error: DecodeIssue.unexpectedArity(expected: 1, actual: args.count))
            return nil
        }

        do {
            return try decodeValue(args[0], as: T.self)
        } catch {
            actor.reportDecodeError(eventName: eventName, error: error)
            return nil
        }
    }

    func decodePair<A: Decodable, B: Decodable>(args: [JSONValue], as _: (A, B).Type) -> (A, B)? {
        guard args.count == 2 else {
            actor.reportDecodeError(eventName: eventName, error: DecodeIssue.unexpectedArity(expected: 2, actual: args.count))
            return nil
        }

        do {
            let first = try decodeValue(args[0], as: A.self)
            let second = try decodeValue(args[1], as: B.self)
            return (first, second)
        } catch {
            actor.reportDecodeError(eventName: eventName, error: error)
            return nil
        }
    }

    func decodeTriple<A: Decodable, B: Decodable, C: Decodable>(args: [JSONValue], as _: (A, B, C).Type) -> (A, B, C)? {
        guard args.count == 3 else {
            actor.reportDecodeError(eventName: eventName, error: DecodeIssue.unexpectedArity(expected: 3, actual: args.count))
            return nil
        }

        do {
            let first = try decodeValue(args[0], as: A.self)
            let second = try decodeValue(args[1], as: B.self)
            let third = try decodeValue(args[2], as: C.self)
            return (first, second, third)
        } catch {
            actor.reportDecodeError(eventName: eventName, error: error)
            return nil
        }
    }

    func decodeZero(args: [JSONValue]) -> Bool {
        if args.isEmpty {
            return true
        }
        actor.reportDecodeError(eventName: eventName, error: DecodeIssue.unexpectedArity(expected: 0, actual: args.count))
        return false
    }

    private func decodeValue<T: Decodable>(_ value: JSONValue, as _: T.Type) throws -> T {
        let data = try JSONEncoder().encode(value)
        return try JSONDecoder().decode(T.self, from: data)
    }
}

private enum DecodeIssue: Error, LocalizedError {
    case unexpectedArity(expected: Int, actual: Int)

    var errorDescription: String? {
        switch self {
        case .unexpectedArity(let expected, let actual):
            return "expected \(expected) args, received \(actual)"
        }
    }
}
