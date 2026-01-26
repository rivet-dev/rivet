import Foundation

enum ActorQuery: Sendable {
    case getForId(name: String, actorId: String)
    case getForKey(name: String, key: [String])
    case getOrCreateForKey(name: String, key: [String], input: AnyEncodable?, region: String?)
    case create(name: String, key: [String], input: AnyEncodable?, region: String?)

    var name: String {
        switch self {
        case .getForId(let name, _):
            return name
        case .getForKey(let name, _):
            return name
        case .getOrCreateForKey(let name, _, _, _):
            return name
        case .create(let name, _, _, _):
            return name
        }
    }
}

struct ActorOutput: Decodable, Sendable {
    let actorId: String
    let name: String
    let key: String?
    let error: JSONValue?

    enum CodingKeys: String, CodingKey {
        case actorId = "actor_id"
        case name
        case key
        case error
    }
}

struct ActorListResponse: Decodable, Sendable {
    let actors: [ActorOutput]
}

struct ActorCreateResponse: Decodable, Sendable {
    let actor: ActorOutput
}

struct ActorGetOrCreateResponse: Decodable, Sendable {
    let actor: ActorOutput
    let created: Bool
}

struct ActorKvGetResponse: Decodable, Sendable {
    let value: String?
}
