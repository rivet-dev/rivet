import RivetKitClient
import RivetKitSwiftUI
import SwiftUI

@main
struct HelloWorldApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
                .rivetKit(endpoint: "http://localhost:6420")
        }
    }
}
