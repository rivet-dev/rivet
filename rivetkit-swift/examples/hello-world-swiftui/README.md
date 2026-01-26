# Hello World (SwiftUI)

A minimal RivetKit example that pairs a simple server with a SwiftUI app. The counter broadcasts updates in real time, so multiple app instances stay in sync.

## Server

```bash
cd rivetkit-swift/examples/hello-world-swiftui/server
pnpm install
pnpm dev
```

The server listens on `http://127.0.0.1:8787` and exposes RivetKit at `http://127.0.0.1:8787/api/rivet`.

## SwiftUI App (macOS)

Open the Xcode project and run it:

```bash
cd rivetkit-swift/examples/hello-world-swiftui/client
open HelloWorldApp.xcodeproj
```

The app connects to `http://127.0.0.1:8787/api/rivet` by default. Override this if you are running the server elsewhere:

```bash
RIVET_ENDPOINT=http://<host-ip>:8787/api/rivet
```

Set the environment variable in Xcode's scheme (Run > Arguments > Environment Variables) or when using `swift run`.

## Notes

- The server uses `registry.serve()` for a fetch-handler setup.
- The SwiftUI client uses `@Actor` and listens for `newCount` to update the UI.
- Use the Actor Key text field to connect to a different actor instance.
- For a CLI Swift example, see `examples/hello-world-cli`.
