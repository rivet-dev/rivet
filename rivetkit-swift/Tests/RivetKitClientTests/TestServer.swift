import Foundation
import RivetKitClient

struct TestServerInfo: Decodable, Sendable {
    let endpoint: String
    let namespace: String
    let runnerName: String
}

actor TestServer {
    static let shared = TestServer()

    private var process: Process?
    private var info: TestServerInfo?
    private var activeClients = 0
    private var startTask: Task<TestServerInfo, Error>?

    func start() async throws -> TestServerInfo {
        activeClients += 1
        if let info {
            return info
        }
        if let startTask {
            return try await startTask.value
        }
        let task = Task { try await startServerProcess() }
        startTask = task
        do {
            let info = try await task.value
            startTask = nil
            return info
        } catch {
            startTask = nil
            activeClients = max(0, activeClients - 1)
            throw error
        }
    }

    private func startServerProcess() async throws -> TestServerInfo {
        if let info {
            return info
        }
        let repoRoot = try resolveRepoRoot()
        let distPath = repoRoot.appendingPathComponent("rivetkit-typescript/packages/rivetkit/dist/tsup/serve-test-suite/mod.js")
        try await ensureBuildArtifacts(in: repoRoot, serveTestSuitePath: distPath)

        let process = Process()
        process.currentDirectoryURL = repoRoot
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = ["node", distPath.path]

        let outputPipe = Pipe()
        process.standardOutput = outputPipe
        process.standardError = outputPipe

        try process.run()

        do {
            let parsedInfo = try await readFirstInfo(from: outputPipe.fileHandleForReading)
            self.process = process
            self.info = parsedInfo
            return parsedInfo
        } catch {
            throw error
        }
    }

    func stop() {
        guard activeClients > 0 else {
            return
        }
        activeClients -= 1
        guard activeClients == 0 else {
            return
        }
        guard let process else {
            return
        }
        process.terminate()
        process.waitUntilExit()
        self.process = nil
        self.info = nil
    }

    private func resolveRepoRoot() throws -> URL {
        let fileURL = URL(fileURLWithPath: #filePath)
        let packageRoot = fileURL.deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
        let repoRoot = packageRoot.deletingLastPathComponent()
        guard FileManager.default.fileExists(atPath: repoRoot.appendingPathComponent("rivetkit-typescript").path) else {
            throw InternalError("failed to locate repo root")
        }
        return repoRoot
    }

    private func readFirstInfo(from handle: FileHandle) async throws -> TestServerInfo {
        final class BufferBox: @unchecked Sendable {
            // Safe because this buffer is only mutated by the readability handler.
            var data = Data()
        }

        let bufferBox = BufferBox()
        return try await withCheckedThrowingContinuation { continuation in
            handle.readabilityHandler = { fileHandle in
                let data = fileHandle.availableData
                if data.isEmpty {
                    return
                }
                bufferBox.data.append(data)
                if let range = bufferBox.data.firstRange(of: Data([0x0A])) {
                    let lineData = bufferBox.data.subdata(in: bufferBox.data.startIndex..<range.lowerBound)
                    handle.readabilityHandler = { fileHandle in
                        _ = fileHandle.availableData
                    }
                    do {
                        let info = try JSONDecoder().decode(TestServerInfo.self, from: lineData)
                        continuation.resume(returning: info)
                    } catch {
                        continuation.resume(throwing: error)
                    }
                }
            }
        }
    }

    private func runProcess(in directory: URL, command: String, arguments: [String]) async throws {
        let process = Process()
        process.currentDirectoryURL = directory
        process.executableURL = URL(fileURLWithPath: command)
        process.arguments = arguments
        let outputPipe = Pipe()
        process.standardOutput = outputPipe
        process.standardError = outputPipe
        try process.run()
        process.waitUntilExit()
        if process.terminationStatus != 0 {
            let data = outputPipe.fileHandleForReading.readDataToEndOfFile()
            let output = String(data: data, encoding: .utf8) ?? ""
            throw InternalError("command failed: \(output)")
        }
    }

    private func ensureBuildArtifacts(in repoRoot: URL, serveTestSuitePath: URL) async throws {
        let nodeModules = repoRoot.appendingPathComponent("node_modules")
        if !FileManager.default.fileExists(atPath: nodeModules.path) {
            try await runProcess(
                in: repoRoot,
                command: "/usr/bin/env",
                arguments: ["pnpm", "install"]
            )
        }

        let requiredArtifacts: [(URL, [String])] = [
            (
                repoRoot.appendingPathComponent("shared/typescript/virtual-websocket/dist/mod.js"),
                ["pnpm", "--filter", "@rivetkit/virtual-websocket", "build"]
            ),
            (
                repoRoot.appendingPathComponent("engine/sdks/typescript/runner-protocol/dist/mod.js"),
                ["pnpm", "--filter", "@rivetkit/engine-runner-protocol", "build"]
            ),
            (
                repoRoot.appendingPathComponent("engine/sdks/typescript/runner/dist/mod.js"),
                ["pnpm", "--filter", "@rivetkit/engine-runner", "build"]
            ),
        ]

        for (artifact, command) in requiredArtifacts where !FileManager.default.fileExists(atPath: artifact.path) {
            try await runProcess(
                in: repoRoot,
                command: "/usr/bin/env",
                arguments: command
            )
        }

        let serveTestSuiteSource = repoRoot.appendingPathComponent("rivetkit-typescript/packages/rivetkit/src/serve-test-suite/mod.ts")
        let registrySource = repoRoot.appendingPathComponent("rivetkit-typescript/packages/rivetkit/fixtures/driver-test-suite/registry.ts")
        let rejectSource = repoRoot.appendingPathComponent("rivetkit-typescript/packages/rivetkit/fixtures/driver-test-suite/reject-connection.ts")
        let connStateSource = repoRoot.appendingPathComponent("rivetkit-typescript/packages/rivetkit/fixtures/driver-test-suite/conn-state.ts")
        if isArtifactStale(artifact: serveTestSuitePath, sources: [serveTestSuiteSource, registrySource, rejectSource, connStateSource]) {
            try await runProcess(
                in: repoRoot,
                command: "/usr/bin/env",
                arguments: ["pnpm", "--filter", "rivetkit", "build:schema"]
            )
            try await runProcess(
                in: repoRoot,
                command: "/usr/bin/env",
                arguments: ["env", "FAST_BUILD=1", "pnpm", "--filter", "rivetkit", "build"]
            )
        }
    }

    private func isArtifactStale(artifact: URL, sources: [URL]) -> Bool {
        guard FileManager.default.fileExists(atPath: artifact.path) else {
            return true
        }
        guard let artifactDate = try? FileManager.default.attributesOfItem(atPath: artifact.path)[.modificationDate] as? Date else {
            return false
        }
        for source in sources {
            guard let sourceDate = try? FileManager.default.attributesOfItem(atPath: source.path)[.modificationDate] as? Date else {
                continue
            }
            if artifactDate < sourceDate {
                return true
            }
        }
        return false
    }
}
