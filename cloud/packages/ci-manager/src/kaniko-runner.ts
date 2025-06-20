import { BuildStore } from "./build-store";
import { mkdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname } from "node:path";
import { runDockerBuild } from "./executors/docker";
import { runRivetBuild } from "./executors/rivet";

export async function runKanikoBuild(
	buildStore: BuildStore,
	serverUrl: string,
	buildId: string,
): Promise<void> {
	const build = buildStore.getBuild(buildId);
	if (!build) {
		throw new Error(`Build ${buildId} not found`);
	}

	await mkdir(dirname(build.contextPath!), { recursive: true });

	buildStore.updateStatus(buildId, {
		type: "running",
		data: {
			noRunner: {}
		}
	});

	const executionMode = process.env.KANIKO_EXECUTION_MODE || "docker";
	buildStore.addLog(buildId, `Using execution mode: ${executionMode}`);

	if (executionMode === "rivet") {
		await runRivetBuild(buildStore, serverUrl, buildId);
	} else {
		await runDockerBuild(buildStore, serverUrl, buildId);
	}

	// Add upload validation check
	await validateBuildUpload(buildStore, buildId);
}

async function validateBuildUpload(
	buildStore: BuildStore,
	buildId: string,
): Promise<void> {
	const build = buildStore.getBuild(buildId);
	if (!build) {
		buildStore.updateStatus(buildId, {
			type: "failure",
			data: { reason: "Build not found" },
		});
		return;
	}

	if (existsSync(build.outputPath)) {
		buildStore.addLog(buildId, "Build upload validated successfully");
	} else {
		buildStore.updateStatus(buildId, {
			type: "failure",
			data: {
				reason: "Build output file was not uploaded after waiting",
			},
		});
	}
}
