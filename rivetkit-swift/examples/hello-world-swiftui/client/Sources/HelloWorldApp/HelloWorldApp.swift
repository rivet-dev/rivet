import RivetKitSwiftUI
import SwiftUI

@main
struct HelloWorldApp: App {
    var body: some Scene {
        WindowGroup {
            let endpoint = ProcessInfo.processInfo.environment["RIVET_ENGINE"]
                ?? ProcessInfo.processInfo.environment["RIVET_ENDPOINT"]
                ?? "http://127.0.0.1:8787/api/rivet"
            ContentView(endpoint: endpoint)
                .rivetKit(endpoint)
        }
    }
}
