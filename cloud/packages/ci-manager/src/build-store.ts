import { BuildInfo, BuildEvent, Status } from "./types";
import { randomUUID } from "crypto";
import { mkdir, rm } from "fs/promises";
import { join, dirname } from "path";
import { createNanoEvents } from "nanoevents";

export class BuildStore {
	private builds = new Map<string, BuildInfo>();
	private tempDir: string;
	public emitter = createNanoEvents<{
		"build-event": (buildId: string, event: BuildEvent) => void;
		"status-change": (buildId: string, status: Status) => void;
	}>();

	constructor(tempDir: string = "/tmp/ci-builds") {
		this.tempDir = tempDir;
	}

	async init() {
		await mkdir(this.tempDir, { recursive: true });
	}

	createBuild(buildName: string, dockerfilePath: string, environmentId: string): string {
		const id = randomUUID();
		const contextPath = join(this.tempDir, id, "context.tar.gz");
		const outputPath = join(this.tempDir, id, "output.tar.gz");

		const build: BuildInfo = {
			id,
			status: { type: "starting", data: {} },
			buildName,
			dockerfilePath,
			environmentId,
			contextPath,
			outputPath,
			events: [],
			createdAt: new Date(),
		};

		// Set up 10-minute cleanup timeout
		build.cleanupTimeout = setTimeout(
			() => {
				this.cleanupBuild(id, "timeout");
			},
			10 * 60 * 1000,
		); // 10 minutes

		this.builds.set(id, build);
		return id;
	}

	getBuild(id: string): BuildInfo | undefined {
		return this.builds.get(id);
	}

	updateStatus(id: string, status: Status) {
		const build = this.builds.get(id);
		if (
			build &&
			build.status.type !== "success" &&
			build.status.type !== "failure"
		) {
			build.status = status;
			const event = { type: "status", data: status } as BuildEvent;
			build.events.push(event);
			this.emitter.emit("build-event", id, event);
			this.emitter.emit("status-change", id, status);
			console.log(`[${id}] status: ${JSON.stringify(status)}`);
		}
	}

	addLog(id: string, line: string) {
		console.log(`[${id}] ${line}`);
		const build = this.builds.get(id);
		if (build) {
			const event: BuildEvent = { type: "log", data: { line } };
			build.events.push(event);
			this.emitter.emit("build-event", id, event);
		}
	}

	setContainerProcess(id: string, process: any) {
		const build = this.builds.get(id);
		if (build) {
			build.containerProcess = process;
		}
	}

	getContextPath(id: string): string | undefined {
		return this.builds.get(id)?.contextPath;
	}

	getOutputPath(id: string): string | undefined {
		return this.builds.get(id)?.outputPath;
	}

	markDownloaded(id: string) {
		const build = this.builds.get(id);
		if (build) {
			build.downloadedAt = new Date();
			// Trigger cleanup after download
			setTimeout(() => {
				this.cleanupBuild(id, "downloaded");
			}, 1000); // Small delay to ensure download is complete
		}
	}

	private async cleanupBuild(id: string, reason: "timeout" | "downloaded") {
		const build = this.builds.get(id);
		if (!build) return;

		console.log(`Cleaning up build ${id} (reason: ${reason})`);

		try {
			// Clear the timeout if it exists
			if (build.cleanupTimeout) {
				clearTimeout(build.cleanupTimeout);
			}

			// Remove build directory and all files
			if (build.contextPath) {
				const buildDir = dirname(build.contextPath);
				try {
					await rm(buildDir, { recursive: true, force: true });
					console.log(`Removed build directory: ${buildDir}`);
				} catch (error) {
					console.warn(
						`Failed to remove build directory ${buildDir}:`,
						error,
					);
				}
			}

			// Remove from memory
			this.builds.delete(id);
			console.log(`Build ${id} cleaned up successfully`);
		} catch (error) {
			console.error(`Error cleaning up build ${id}:`, error);
		}
	}

	// Manual cleanup method for testing or admin use
	async manualCleanup(id: string) {
		await this.cleanupBuild(id, "downloaded");
	}
}
