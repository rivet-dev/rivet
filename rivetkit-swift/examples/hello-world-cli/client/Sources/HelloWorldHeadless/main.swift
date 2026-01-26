import Foundation
import RivetKitClient

@main
struct HelloWorldHeadless {
    static func main() async {
        let endpoint = ProcessInfo.processInfo.environment["RIVET_ENGINE"]
            ?? ProcessInfo.processInfo.environment["RIVET_ENDPOINT"]
            ?? "http://127.0.0.1:8787/api/rivet"
        let key = resolveActorKey()

        print("Connecting to \(endpoint)")
        print("Actor key: \(key)")

        do {
            let client = try RivetKitClient(endpoint: endpoint)
            let handle = client.getOrCreate("counter", [key])
            let connection = handle.connect()

            _ = await connection.onStatusChange { status in
                print("status: \(status.rawValue)")
            }

            _ = await connection.onError { error in
                print("error: \(error.group).\(error.code): \(error.message)")
            }

            _ = await connection.on("newCount") { args in
                if let newValue = decodeCount(from: args) {
                    print("event newCount: \(newValue)")
                }
            }

            let initial: Int = try await handle.action("getCount")
            print("initial count: \(initial)")

            for _ in 0..<3 {
                let newValue: Int = try await connection.action("increment", arg: 1)
                print("increment -> \(newValue)")
            }

            try? await Task.sleep(nanoseconds: 3_000_000_000)

            await connection.dispose()
            await client.dispose()
        } catch {
            print("error: \(error)")
        }
    }

    private static func resolveActorKey() -> String {
        if let envKey = ProcessInfo.processInfo.environment["RIVET_ACTOR_KEY"], !envKey.isEmpty {
            return envKey
        }

        let args = Array(CommandLine.arguments.dropFirst())
        if let flagIndex = args.firstIndex(of: "--key"), flagIndex + 1 < args.count {
            return args[flagIndex + 1]
        }
        if let first = args.first, !first.hasPrefix("-") {
            return first
        }

        return "swift-cli"
    }

    private static func decodeCount(from args: [JSONValue]) -> Int? {
        guard let first = args.first else { return nil }
        switch first {
        case .number(let number):
            return intValue(from: number)
        case .object(let object):
            if case .number(let number)? = object["count"] {
                return intValue(from: number)
            }
            return nil
        default:
            return nil
        }
    }

    private static func intValue(from number: JSONNumber) -> Int {
        switch number {
        case .int(let value):
            return Int(value)
        case .double(let value):
            return Int(value)
        }
    }
}
