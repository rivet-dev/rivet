import { spawn } from "node:child_process";
import { BuildStore } from "../build-store";

export async function runDockerBuild(
	buildStore: BuildStore,
	serverUrl: string,
	buildId: string,
): Promise<void> {
	const build = buildStore.getBuild(buildId);
	if (!build) {
		throw new Error(`Build ${buildId} not found`);
	}

	const contextUrl = `${serverUrl}/builds/${buildId}/kaniko/context.tar.gz`;
	const outputUrl = `${serverUrl}/builds/${buildId}/kaniko/output.tar.gz`;

	const kanikoArgs = [
		"run",
		"--rm",
		"--network=host",
		"-e",
		`CONTEXT_URL=${contextUrl}`,
		"-e",
		`OUTPUT_URL=${outputUrl}`,
		"-e",
		`DESTINATION=${buildId}:latest`,
		"-e",
		`DOCKERFILE_PATH=${build.dockerfilePath}`,
		"ci-runner",
	];

	buildStore.addLog(
		buildId,
		`Starting kaniko with args: docker ${kanikoArgs.join(" ")}`,
	);

	return new Promise<void>((resolve, reject) => {
		const dockerProcess = spawn("docker", kanikoArgs, {
			stdio: ["pipe", "pipe", "pipe"],
		});

		buildStore.setContainerProcess(buildId, dockerProcess);

		dockerProcess.stdout?.on("data", (data) => {
			const lines = data
				.toString()
				.split("\n")
				.filter((line: string) => line.trim());
			lines.forEach((line: string) => {
				buildStore.addLog(buildId, `[kaniko] ${line}`);
			});
		});

		dockerProcess.stderr?.on("data", (data) => {
			const lines = data
				.toString()
				.split("\n")
				.filter((line: string) => line.trim());
			lines.forEach((line: string) => {
				buildStore.addLog(buildId, `[kaniko-error] ${line}`);
			});
		});

		dockerProcess.on("close", (code) => {
			buildStore.addLog(buildId, `Docker process closed with exit code: ${code}`);
			buildStore.updateStatus(buildId, { type: "finishing", data: {} });

			if (code === 0) {
				resolve();
			} else {
				buildStore.updateStatus(buildId, {
					type: "failure",
					data: { reason: `Container exited with code ${code}` },
				});
				reject(new Error(`Container exited with code ${code}`));
			}
		});

		dockerProcess.on("spawn", () => {
			buildStore.addLog(buildId, "Docker process spawned successfully");
		});

		dockerProcess.on("error", (error) => {
			buildStore.addLog(buildId, `Docker process error: ${error.message}`);
			buildStore.updateStatus(buildId, {
				type: "failure",
				data: { reason: `Failed to start kaniko: ${error.message}` },
			});
			reject(error);
		});
	});
}


