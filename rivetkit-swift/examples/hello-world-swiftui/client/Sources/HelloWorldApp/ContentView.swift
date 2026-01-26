import RivetKitSwiftUI
import SwiftUI

struct ContentView: View {
    @State private var keyInput = "swift-ui"

    var body: some View {
        VStack(spacing: 16) {
            Text("RivetKit SwiftUI Counter")
                .font(.title2)
                .fontWeight(.semibold)

            VStack(alignment: .leading, spacing: 8) {
                Text("Actor Key")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                TextField("my-counter", text: $keyInput)
                    .textFieldStyle(.roundedBorder)
            }
            .frame(maxWidth: 280)

            CounterPanel(key: keyInput)
                .id(keyInput)
        }
        .padding(32)
    }
}

private struct CounterPanel: View {
    let key: String

    @Actor("counter", key: ["default"]) private var counter
    @State private var count = 0
    @State private var lastError: String?

    init(key: String) {
        self.key = key
        _counter = Actor("counter", key: [key])
    }

    var body: some View {
        VStack(spacing: 20) {
            Text("\(count)")
                .font(.system(size: 64, weight: .bold, design: .rounded))

            Button("Increment") {
                counter.send("increment", 1)
            }
            .buttonStyle(.borderedProminent)
            .disabled(!counter.isConnected)

            Text("Status: \(counter.connStatus.rawValue)")
                .font(.caption)
                .foregroundStyle(.secondary)

            if let error = lastError {
                Text(error)
                    .font(.caption)
                    .foregroundStyle(.red)
                    .multilineTextAlignment(.center)
            }
        }
        .task {
            count = (try? await counter.action("getCount")) ?? 0
        }
        .onActorEvent(counter, "newCount") { (newCount: Int) in
            count = newCount
        }
        .onActorError(counter) { error in
            lastError = "\(error.group).\(error.code): \(error.message)"
        }
    }
}
