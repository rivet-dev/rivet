import Foundation
import RivetKitClient

@main
struct HelloWorldHeadless {
    static func main() async {
        let config = try! ClientConfig(endpoint: "http://localhost:3000/api/rivet")
        let key = resolveActorKey()

        print("Connecting to \(config.endpoint)")
        print("Actor key: \(key)")

        do {
            let client = RivetKitClient(config: config)
            let handle = client.getOrCreate("counter", [key])
            let connection = handle.connect()

            // Monitor status changes using AsyncStream
            let statusTask = Task {
                for await status in await connection.statusChanges() {
                    print("status: \(status.rawValue)")
                }
            }

            // Monitor errors using AsyncStream
            let errorTask = Task {
                for await error in await connection.errors() {
                    print("error: \(error.group).\(error.code): \(error.message)")
                }
            }

            // Subscribe to newCount events using AsyncStream
            let eventTask = Task {
                for await newValue in await connection.events("newCount", as: Int.self) {
                    print("event newCount: \(newValue)")
                }
            }

            let initial: Int = try await handle.action("getCount")
            print("initial count: \(initial)")

            for _ in 0..<3 {
                let newValue: Int = try await connection.action("increment", 1)
                print("increment -> \(newValue)")
            }

            try? await Task.sleep(nanoseconds: 3_000_000_000)

            statusTask.cancel()
            errorTask.cancel()
            eventTask.cancel()
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

}
