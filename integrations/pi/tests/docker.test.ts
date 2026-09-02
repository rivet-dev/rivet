import { execFile as execFileCallback } from "node:child_process";
import { randomUUID } from "node:crypto";
import { promisify } from "node:util";
import type { Sandbox } from "@rivet-dev/sandbox-adapter";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { createSandboxTools } from "../src/index.js";

const execFile = promisify(execFileCallback);
const runDockerTests = process.env.PI_DOCKER_TESTS === "1";
const containerName = `rivet-pi-test-${process.pid}-${randomUUID().slice(0, 8)}`;

async function docker(...args: string[]) {
	return execFile("docker", args, {
		encoding: "utf8",
		maxBuffer: 4 * 1024 * 1024,
	});
}

function shellQuote(value: string): string {
	return `'${value.replaceAll("'", `'"'"'`)}'`;
}

function dockerSandbox(): Sandbox {
	const command = async (script: string, cwd = "/workspace") => {
		try {
			const result = await docker(
				"exec",
				"--workdir",
				cwd,
				containerName,
				"sh",
				"-lc",
				script,
			);
			return { exitCode: 0, stdout: result.stdout, stderr: result.stderr };
		} catch (error) {
			const result = error as { code?: number; stdout?: string; stderr?: string };
			return {
				exitCode: typeof result.code === "number" ? result.code : 1,
				stdout: result.stdout ?? "",
				stderr: result.stderr ?? String(error),
			};
		}
	};
	return {
		binding: { provider: "docker-test", containerName },
		cwd: "/workspace",
		async exec(script, options) {
			const result = await command(script, options?.cwd);
			if (result.stdout) {
				options?.onOutput?.({
					sequence: 0,
					stream: "stdout",
					data: Buffer.from(result.stdout),
				});
			}
			return { ...result, outcome: "exited" };
		},
		async spawn() {
			throw new Error("Docker test sandbox does not implement spawn");
		},
		async readFile(path) {
			const result = await command(`base64 ${shellQuote(path)}`);
			if (result.exitCode !== 0) throw new Error(result.stderr);
			return Buffer.from(result.stdout.replace(/\s/g, ""), "base64");
		},
		async writeFile(path, content) {
			const encoded = Buffer.from(content).toString("base64");
			const result = await command(
				`mkdir -p $(dirname ${shellQuote(path)}) && printf %s ${shellQuote(encoded)} | base64 -d > ${shellQuote(path)}`,
			);
			if (result.exitCode !== 0) throw new Error(result.stderr);
		},
		async stat(path) {
			const result = await command(
				`if test -d ${shellQuote(path)}; then printf directory; elif test -f ${shellQuote(path)}; then printf file; else printf other; fi`,
			);
			return { type: result.stdout as "file" | "directory" | "other", size: 0 };
		},
		async readdir(path) {
			const result = await command(`ls -1A ${shellQuote(path)}`);
			if (result.exitCode !== 0) throw new Error(result.stderr);
			return result.stdout.trim() ? result.stdout.trim().split("\n") : [];
		},
		async exists(path) {
			return (await command(`test -e ${shellQuote(path)}`)).exitCode === 0;
		},
		async mkdir(path) {
			const result = await command(`mkdir -p ${shellQuote(path)}`);
			if (result.exitCode !== 0) throw new Error(result.stderr);
		},
		async remove(path, options) {
			const flags = options?.recursive ? "-r" : "";
			const force = options?.force ? "-f" : "";
			const result = await command(
				`rm ${flags} ${force} -- ${shellQuote(path)}`,
			);
			if (result.exitCode !== 0) throw new Error(result.stderr);
		},
	};
}

describe.runIf(runDockerTests)("Pi Docker sandbox integration", () => {
	beforeAll(async () => {
		await docker(
			"run",
			"--detach",
			"--rm",
			"--name",
			containerName,
			"node:22-alpine",
			"sh",
			"-lc",
			"apk add --no-cache ripgrep >/dev/null && mkdir -p /workspace && sleep 300",
		);
		const deadline = Date.now() + 120_000;
		while (true) {
			try {
				await docker(
					"exec",
					containerName,
					"sh",
					"-lc",
					"test -d /workspace && command -v rg >/dev/null",
				);
				break;
			} catch (error) {
				if (Date.now() >= deadline) throw error;
				await new Promise((resolve) => setTimeout(resolve, 250));
			}
		}
	}, 120_000);

	afterAll(async () => {
		await docker("rm", "--force", containerName).catch(() => {});
	}, 30_000);

	it("runs Pi tools inside the container", async () => {
		const tools = createSandboxTools(dockerSandbox());
		const byName = (name: string) =>
			tools.find((candidate) => candidate.name === name)!;
		const context = {
			sessionManager: {
				getSessionId: () => "session-1",
				getSessionFile: () => undefined,
			},
			thinkingLevel: "medium",
		} as never;

		await byName("write").execute(
			"write-1",
			{ path: "hello.txt", content: "hello from Docker\n" },
			undefined,
			undefined,
			context,
		);
		const read = await byName("read").execute(
			"read-1",
			{ path: "hello.txt" },
			undefined,
			undefined,
			context,
		);
		const grep = await byName("grep").execute(
			"grep-1",
			{ pattern: "Docker" },
			undefined,
			undefined,
			context,
		);

		expect(read.content[0]).toMatchObject({ text: "hello from Docker\n" });
		expect(grep.content[0]).toMatchObject({
			text: expect.stringContaining("hello.txt:1:hello from Docker"),
		});
	});
});
