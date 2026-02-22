import { spawn, ChildProcess } from "node:child_process";
import { readdir, readFile, mkdir, writeFile, access, rm } from "node:fs/promises";
import { createWriteStream } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { parseArgs } from "node:util";
import { tmpdir } from "node:os";
import { z } from "zod";
import { SandboxAgent, type ItemDeltaData } from "sandbox-agent";

// --- Terminal styling ---

const { fmt, STDOUT_IS_TTY } = (() => {
	const STDOUT_IS_TTY = !!process.stdout.isTTY;
	const enabled =
		STDOUT_IS_TTY &&
		process.env.TERM !== "dumb" &&
		!("NO_COLOR" in process.env);

	const esc = (code: string) => (enabled ? `\u001b[${code}m` : "");
	const wrap = (text: string, code: string) => (enabled ? `${esc(code)}${text}${esc("0")}` : text);

	const fmt = {
		bold: (s: string) => wrap(s, "1"),
		dim: (s: string) => wrap(s, "2"),
		gray: (s: string) => wrap(s, "90"),
		cyan: (s: string) => wrap(s, "36"),
		yellow: (s: string) => wrap(s, "33;1"),
		red: (s: string) => wrap(s, "31;1"),
		green: (s: string) => wrap(s, "32;1"),
	};

	return { fmt, STDOUT_IS_TTY };
})();

// --- Config ---

const {
	ROOT,
	PACKAGE_ROOT,
	REPO_ROOT,
	EVALS_DIR,
	RESULTS_DIR,
	JUDGE_SYSTEM_PATH,
	SKILLS_DIR,
	FRICTION_LOG_FILENAME,
} = (() => {
	const ROOT = dirname(new URL(import.meta.url).pathname);
	const PACKAGE_ROOT = resolve(ROOT, "..");
	const REPO_ROOT = resolve(PACKAGE_ROOT, "../..");
	const EVALS_DIR = join(PACKAGE_ROOT, "evals");
	const RESULTS_DIR = join(PACKAGE_ROOT, "results");
	const JUDGE_SYSTEM_PATH = join(ROOT, "judge-system.md");
	const SKILLS_DIR = join(REPO_ROOT, "website/dist/metadata/skills");
	const FRICTION_LOG_FILENAME = "FRICTION.md";

	return {
		ROOT,
		PACKAGE_ROOT,
		REPO_ROOT,
		EVALS_DIR,
		RESULTS_DIR,
		JUDGE_SYSTEM_PATH,
		SKILLS_DIR,
		FRICTION_LOG_FILENAME,
	};
})();

// --- CLI args ---

const {
	AGENT,
	AGENT_SYSTEM,
	EVAL_NAME,
	MODEL,
	VARIANT,
	JUDGE,
	JUDGE_MODEL,
	JUDGE_VARIANT,
	KEEP_TMP,
} = (() => {
	const { values: args } = parseArgs({
		options: {
			agent: { type: "string", default: "claude" },
			"agent-system": { type: "string", default: "" },
			eval: { type: "string", default: "" },
			model: { type: "string", default: "" },
			variant: { type: "string", default: "" },
			judge: { type: "string", default: "claude" },
			"judge-model": { type: "string", default: "" },
			"judge-variant": { type: "string", default: "" },
			"keep-tmp": { type: "boolean", default: false },
			help: { type: "boolean", short: "h", default: false },
		},
	});

	if (args.help) {
		console.log(`Usage: npx tsx src/index.ts [options]

Options:
  --agent  claude|codex   Agent for project generation (default: claude)
  --agent-system <text>   Extra system prompt for the project agent (supports {{TMP_DIR}} and {{FRICTION_LOG_PATH}})
  --eval   <name>         Eval to run (required)
  --model  <name>         Model for agent under test (default: opus for claude, 5.3 for codex)
  --variant <name>        Model variant (default: high for codex, empty otherwise)
  --judge  claude|codex   Agent for judge (default: claude)
  --judge-model <name>    Model for judge (default: opus for claude, 5.3 for codex)
  --judge-variant <name>  Model variant for judge (default: high for codex, empty otherwise)
  --keep-tmp              Keep temp dirs even on success
  -h, --help              Show this help`);
		process.exit(0);
	}

	const AGENT = args.agent!;
	const AGENT_SYSTEM = args["agent-system"]!;
	const EVAL_NAME = args.eval!;
	let MODEL = args.model!.trim();
	let VARIANT = args.variant!.trim();
	const JUDGE = args.judge!;
	let JUDGE_MODEL = args["judge-model"]!.trim();
	let JUDGE_VARIANT = args["judge-variant"]!.trim();
	const KEEP_TMP = args["keep-tmp"]!;

	if (!EVAL_NAME) {
		console.error("Missing required flag: pass --eval <name>.");
		process.exit(1);
	}

	const applyDefaults = (
		agent: string,
		model: string,
		variant: string,
	): { model: string; variant: string } => {
		if (agent === "codex") {
			const nextModel = model || "5.3";
			const nextVariant = variant || "high";
			return { model: nextModel, variant: nextVariant };
		}
		if (agent === "claude") {
			const nextModel = model || "opus";
			return { model: nextModel, variant: variant || "" };
		}
		return { model: model || "", variant: variant || "" };
	};

	({ model: MODEL, variant: VARIANT } = applyDefaults(AGENT, MODEL, VARIANT));
	({ model: JUDGE_MODEL, variant: JUDGE_VARIANT } = applyDefaults(JUDGE, JUDGE_MODEL, JUDGE_VARIANT));

	return {
		AGENT,
		AGENT_SYSTEM,
		EVAL_NAME,
		MODEL,
		VARIANT,
		JUDGE,
		JUDGE_MODEL,
		JUDGE_VARIANT,
		KEEP_TMP,
	};
})();

// --- Prompt helpers ---

const { buildProjectAgentSystemPrompt } = (() => {
	const DEFAULT_BASE = "Create a project in {{TMP_DIR}}. Install dependencies. Do not ask questions; proceed autonomously.";

	function interpolate(template: string, data: Record<string, string>): string {
		let out = template;
		for (const [key, value] of Object.entries(data)) {
			out = out.replaceAll(`{{${key}}}`, value);
		}
		return out;
	}

	function buildProjectAgentSystemPrompt(tmpDir: string): string {
		const frictionLogPath = `${tmpDir}/${FRICTION_LOG_FILENAME}`;

		const base = (AGENT_SYSTEM || "").trim() ? AGENT_SYSTEM : DEFAULT_BASE;
		const baseInterpolated = interpolate(base, {
			TMP_DIR: tmpDir,
			FRICTION_LOG_PATH: frictionLogPath,
		}).trim();

		const friction = `Keep a friction log as you work: append bullet points to ${frictionLogPath} whenever you hit errors, confusing docs/APIs, or need workarounds.`;

		const rivetkitLinking = [
			"RivetKit linking: If you create a package.json that depends on RivetKit packages, set versions to '*' (do not pin).",
			"This repository uses package manager resolutions to redirect these to the local workspace builds.",
			"Examples:",
			'  "rivetkit": "*",',
			'  "@rivetkit/react": "*",',
		].join("\n");

		return ["# System", baseInterpolated, friction, rivetkitLinking].filter(Boolean).join("\n\n");
	}

	return { buildProjectAgentSystemPrompt };
})();

// --- Sandbox agent helpers ---

const { collectTurnText } = (() => {
	type AgentLogSink = {
		writeDelta: (delta: string) => void;
		writeLine: (line: string) => void;
	};

	function logItemParts(
		item: {
			content?: Array<{
				type?: string;
				label?: string;
				detail?: string | null;
				action?: string;
				path?: string;
				name?: string;
				call_id?: string;
			}>;
		},
		sink: AgentLogSink,
	) {
		if (!Array.isArray(item.content)) return;
		for (const part of item.content) {
			if (!part?.type) continue;

			if (part.type === "status") {
				const label = part.label ?? "status";
				const detail = part.detail ? `: ${part.detail}` : "";
				sink.writeLine(`[status] ${label}${detail}`);
			} else if (part.type === "file_ref") {
				const action = part.action ?? "file";
				const path = part.path ?? "";
				sink.writeLine(`[file] ${action} ${path}`);
			} else if (part.type === "tool_call") {
				const name = part.name ?? "tool";
				sink.writeLine(`[tool] ${name}`);
			} else if (part.type === "tool_result") {
				const callId = part.call_id ?? "tool_result";
				sink.writeLine(`[tool_result] ${callId}`);
			}
		}
	}

	async function collectTurnText(
		client: SandboxAgent,
		sessionId: string,
		message: string,
		sink?: AgentLogSink,
		signal?: AbortSignal,
	): Promise<string> {
		const eventStream = await client.streamTurn(sessionId, { message }, undefined, signal);

		let deltaText = "";
		let completedTexts: string[] = [];

		for await (const event of eventStream) {
			if (event.type === "error") {
				const data = event.data as { message: string };
				if (sink) sink.writeLine(`[error] ${data.message}`);
				throw new Error(data.message);
			}

			if (event.type === "session.ended") {
				const data = event.data as { reason: string; message?: string | null };
				if (data.reason === "error") {
					const msg = data.message || "Session ended with error";
					if (sink) sink.writeLine(`[session.ended] ${msg}`);
					throw new Error(msg);
				}
			}

			if (event.type === "agent.unparsed") {
				const data = event.data as { error: string };
				if (sink) sink.writeLine(`[agent.unparsed] ${data.error}`);
			}

			if (event.type === "item.delta") {
				const data = event.data as ItemDeltaData;
				if (data.delta) {
					deltaText += data.delta;
					if (sink) sink.writeDelta(data.delta);
				}
			}

			if (event.type === "item.completed") {
				const data = event.data as {
					item?: {
						kind?: string;
						role?: string;
						content?: Array<{ type?: string; text?: string; label?: string; detail?: string | null }>;
					};
				};
				if (sink && data.item) {
					logItemParts(data.item, sink);
				}
				if (data.item?.role === "assistant" && Array.isArray(data.item.content)) {
					const text = data.item.content
						.filter((p) => p.type === "text" && p.text)
						.map((p) => p.text!)
						.join("");
					if (text) completedTexts.push(text);
				}
			}

			// Auto-approve any permission requests
			if (event.type === "permission.requested") {
				const data = event.data as { permission_id: string };
				try {
					await client.replyPermission(sessionId, data.permission_id, { reply: "always" });
				} catch {
					// Best effort
				}
			}
		}

		// Prefer delta text (streaming), fall back to completed item text
		return deltaText || completedTexts.join("\n");
	}

	return { collectTurnText };
})();

// --- Assert skills are built ---

const { assertSkillsBuilt, assertRivetkitBuilt, loadSkillContent, discoverSkillIds } = (() => {
	async function assertSkillsBuilt(): Promise<void> {
		try {
			await access(SKILLS_DIR);
		} catch {
			console.error(`Skills not found at ${SKILLS_DIR}`);
			console.error(`Run: pnpm build -F rivet-website`);
			process.exit(1);
		}
	}

	async function assertRivetkitBuilt(): Promise<void> {
		const modJs = join(REPO_ROOT, "rivetkit-typescript/packages/rivetkit/dist/tsup/mod.js");
		try {
			await access(modJs);
		} catch {
			console.error(`RivetKit build output not found: ${modJs}`);
			console.error("Build it before running evals:");
			console.error("  pnpm -C rivetkit-typescript/packages/rivetkit build");
			console.error("  pnpm -F rivetkit build");
			process.exit(1);
		}
	}

	async function loadSkillContent(skillId: string): Promise<string> {
		const skillDir = join(SKILLS_DIR, skillId);
		const skillMdPath = join(skillDir, "SKILL.md");

		let skillMd: string;
		try {
			skillMd = await readFile(skillMdPath, "utf-8");
		} catch {
			throw new Error(`Skill file not found: ${skillMdPath}`);
		}

		return `# Skill: ${skillId}\n\n${skillMd.trim()}\n`;
	}

	async function discoverSkillIds(): Promise<string[]> {
		const entries = await readdir(SKILLS_DIR);
		const skillIds: string[] = [];
		for (const entry of entries.sort()) {
			try {
				await access(join(SKILLS_DIR, entry, "SKILL.md"));
				skillIds.push(entry);
			} catch {
				// Not a skill directory
			}
		}
		return skillIds;
	}

	return { assertSkillsBuilt, assertRivetkitBuilt, loadSkillContent, discoverSkillIds };
})();

// --- Resolve eval files ---

const { resolveEvalFiles } = (() => {
	async function resolveEvalFiles(): Promise<{ promptPath: string; judgePath: string }> {
		const caseDir = join(EVALS_DIR, EVAL_NAME);
		const promptPath = join(caseDir, "prompt.md");
		const judgePath = join(caseDir, "judge.md");

		try {
			await access(promptPath);
			await access(judgePath);
		} catch {
			console.error(`Eval not found or missing files: ${caseDir}`);
			console.error("Expected files:");
			console.error(`  ${promptPath}`);
			console.error(`  ${judgePath}`);
			process.exit(1);
		}

		return { promptPath, judgePath };
	}

	return { resolveEvalFiles };
})();

// --- Start dev server and wait for it ---

const { startDevServer, waitForServer, killServer } = (() => {
	type Tail = { stdout: string[]; stderr: string[] };

	function pushTail(tail: string[], chunk: string, maxLines: number): void {
		const lines = chunk.replace(/\r\n/g, "\n").split("\n");
		for (const line of lines) {
			if (!line) continue;
			tail.push(line);
			if (tail.length > maxLines) tail.shift();
		}
	}

	function startDevServer(cwd: string, resultDir: string): {
		proc: ChildProcess;
		stdoutPath: string;
		stderrPath: string;
		tail: Tail;
		closeLogs: () => void;
	} {
		const proc = spawn("npm", ["run", "dev"], {
			cwd,
			stdio: ["ignore", "pipe", "pipe"],
			detached: true,
		});

		const stdoutPath = join(resultDir, "dev-server.stdout.log");
		const stderrPath = join(resultDir, "dev-server.stderr.log");
		const stdoutStream = createWriteStream(stdoutPath, { flags: "a" });
		const stderrStream = createWriteStream(stderrPath, { flags: "a" });
		const tail: Tail = { stdout: [], stderr: [] };

		proc.stdout?.on("data", (buf) => {
			const chunk = String(buf);
			stdoutStream.write(chunk);
			pushTail(tail.stdout, chunk, 200);
		});
		proc.stderr?.on("data", (buf) => {
			const chunk = String(buf);
			stderrStream.write(chunk);
			pushTail(tail.stderr, chunk, 200);
		});

		const closeLogs = () => {
			try { stdoutStream.end(); } catch {}
			try { stderrStream.end(); } catch {}
		};

		return { proc, stdoutPath, stderrPath, tail, closeLogs };
	}

	async function waitForServer(url: string, timeoutMs = 60_000): Promise<void> {
		const start = Date.now();
		while (Date.now() - start < timeoutMs) {
			try {
				const resp = await fetch(url);
				if (resp.ok || resp.status < 500) return;
			} catch {
				// Not ready yet
			}
			await new Promise((r) => setTimeout(r, 1000));
		}
		throw new Error(`Server at ${url} did not become ready within ${timeoutMs / 1000}s`);
	}

	function killServer(proc: ChildProcess): void {
		if (proc.pid) {
			try {
				// Kill the process group since we used detached
				process.kill(-proc.pid, "SIGTERM");
			} catch {
				try {
					proc.kill("SIGTERM");
				} catch {
					// Already dead
				}
			}
		}
	}

	return { startDevServer, waitForServer, killServer };
})();

// --- Judge verdict schema ---

const { VerdictSchema, parseVerdict } = (() => {
	const VerdictSchema = z.object({
		criteria: z.array(z.object({
			name: z.string(),
			pass: z.boolean(),
			reason: z.string(),
		})),
		observations: z.array(z.object({
			summary: z.string(),
			severity: z.enum(["low", "medium", "high"]),
		})),
		friction: z.array(z.object({
			summary: z.string(),
			fix: z.string(),
		})),
		pass: z.boolean(),
		summary: z.string(),
	});

	function extractJson(text: string): unknown | null {
		const fenced = text.match(/```(?:json)?\s*\n([\s\S]*?)\n```/);
		const jsonStr = fenced ? fenced[1] : text;

		try {
			return JSON.parse(jsonStr.trim());
		} catch {
			const match = jsonStr.match(/\{[\s\S]*\}/);
			if (match) {
				try {
					return JSON.parse(match[0]);
				} catch {
					return null;
				}
			}
			return null;
		}
	}

	function parseVerdict(raw: string): {
		verdict: z.infer<typeof VerdictSchema> | null;
		errors: string[];
	} {
		const json = extractJson(raw);
		if (json === null) {
			return { verdict: null, errors: ["Could not extract JSON from judge response"] };
		}

		const result = VerdictSchema.safeParse(json);
		if (!result.success) {
			const errors = result.error.issues.map(
				(issue) => `${issue.path.join(".")}: ${issue.message}`,
			);
			return { verdict: null, errors };
		}

		return { verdict: result.data, errors: [] };
	}

	return { VerdictSchema, parseVerdict };
})();

type Verdict = z.infer<typeof VerdictSchema>;

// --- Main ---

type Result = "PASS" | "FAIL" | "ERROR";

async function main() {
	await assertSkillsBuilt();
	await assertRivetkitBuilt();

	const judgeSystem = await readFile(JUDGE_SYSTEM_PATH, "utf-8");
	const { promptPath, judgePath } = await resolveEvalFiles();

	// Load all skills once upfront
	const skillIds = await discoverSkillIds();

	const allSkillContent: string[] = [];
	for (const skillId of skillIds) {
		allSkillContent.push(await loadSkillContent(skillId));
	}
	const skillsBlock = allSkillContent.join("\n\n---\n\n");

	// Start sandbox-agent daemon
	const fmtModel = (model: string, variant: string) =>
		variant ? `${model} (${variant})` : model;

	console.log(`${fmt.bold("eval:")} ${EVAL_NAME}`);
	console.log(`${fmt.bold("agent:")} ${AGENT} (${fmtModel(MODEL, VARIANT)})`);
	console.log(`${fmt.bold("judge:")} ${JUDGE} (${fmtModel(JUDGE_MODEL, JUDGE_VARIANT)})`);
	console.log("");
	console.log("Starting sandbox-agent daemon...");
	const client = await SandboxAgent.start({ spawn: { log: "silent" } });
	console.log("Daemon ready.\n");

	try {
		const prompt = await readFile(promptPath, "utf-8");
		const judgeTemplate = await readFile(judgePath, "utf-8");

		const resultDir = join(RESULTS_DIR, EVAL_NAME);
		await mkdir(resultDir, { recursive: true });

		const start = Date.now();

		// Create temp project directory
		const tmpDir = join(tmpdir(), `skill-eval-${EVAL_NAME}-${Date.now()}`);
		await mkdir(tmpDir, { recursive: true });
		console.log(`${fmt.bold("tmp:")} ${tmpDir}`);

		// 1) Build agent message from skills + prompt + system instructions
		const systemPrompt = buildProjectAgentSystemPrompt(tmpDir);

		const agentMessage = [
			systemPrompt,
			"",
			"# Skill Documentation\n",
			skillsBlock,
			"\n---\n",
			"# Task\n",
			prompt,
		].join("\n");

		// 2) Run agent to generate the project
		const agentSessionId = `eval-agent-${EVAL_NAME}-${Date.now()}`;
		let response: string;
		console.log(`${fmt.bold("== agent ==")}`);
		try {
			await client.createSession(agentSessionId, {
				agent: AGENT,
				model: MODEL,
				...(VARIANT ? { variant: VARIANT } : {}),
				permissionMode: "bypass",
			});

			const agentLogPath = join(resultDir, "agent.log");
			const agentLogStream = createWriteStream(agentLogPath, { flags: "a" });
			let atLineStart = true;
			const sink = {
				writeDelta: (delta: string) => {
					agentLogStream.write(delta);
					process.stdout.write(delta);
					atLineStart = delta.endsWith("\n");
				},
				writeLine: (line: string) => {
					if (!atLineStart) {
						agentLogStream.write("\n");
						process.stdout.write("\n");
					}
					agentLogStream.write(line + "\n");
					process.stdout.write(line + "\n");
					atLineStart = true;
				},
			};

			const timeout = AbortSignal.timeout(600_000); // 10 minute timeout
			response = await collectTurnText(client, agentSessionId, agentMessage, sink, timeout);
			try { agentLogStream.end(); } catch {}
		} catch (err) {
			const duration = Math.round((Date.now() - start) / 1000);
			console.log(`\n${fmt.red("ERROR")} (agent, ${duration}s)  ${(err as Error).message.slice(0, 200)}`);
			try { await client.terminateSession(agentSessionId); } catch {}
			console.log(`${fmt.bold("results:")} ${RESULTS_DIR}`);
			process.exit(1);
		}

		try { await client.terminateSession(agentSessionId); } catch {}
		await writeFile(join(resultDir, "response.md"), response);

		// Read friction log if it exists
		let frictionLog = "";
		try {
			frictionLog = await readFile(join(tmpDir, FRICTION_LOG_FILENAME), "utf-8");
			await writeFile(join(resultDir, "friction.md"), frictionLog);
		} catch {
			// No friction log created, that's fine
		}

		// 3) Start the dev server
		let serverProc: ChildProcess | null = null;
		let serverLogs: ReturnType<typeof startDevServer> | null = null;
		const devUrl = "http://localhost:5173";
		console.log(`\n${fmt.bold("== dev-server ==")}`);
		try {
			serverLogs = startDevServer(tmpDir, resultDir);
			serverProc = serverLogs.proc;
			await waitForServer(devUrl, 60_000);
		} catch (err) {
			if (serverProc) killServer(serverProc);
			if (serverLogs) serverLogs.closeLogs();
			const duration = Math.round((Date.now() - start) / 1000);
			console.log(`${fmt.red("ERROR")} (server, ${duration}s)  ${(err as Error).message.slice(0, 200)}`);
			console.log(`${fmt.bold("results:")} ${RESULTS_DIR}`);
			process.exit(1);
		}

		// 4) Run the judge, retrying on invalid JSON
		const judgeInput = judgeTemplate.replace(/\{\{URL\}\}/g, devUrl);
		const MAX_JUDGE_ATTEMPTS = 3;

		let verdict: Verdict | null = null;
		let verdictRaw = "";
		let judgeError: Error | null = null;
		let lastValidationErrors: string[] | null = null;

		console.log(`\n${fmt.bold("== judge ==")}`);
		for (let attempt = 1; attempt <= MAX_JUDGE_ATTEMPTS; attempt++) {
			const retryInput = attempt === 1
				? `${judgeSystem}\n\n${judgeInput}`
				: `${judgeSystem}\n\n${judgeInput}\n\nIMPORTANT: Your previous response had invalid JSON. Validation errors:\n${lastValidationErrors!.map((e) => `- ${e}`).join("\n")}\n\nYou MUST respond with valid JSON matching the schema exactly. Every field is required.`;

			const judgeSessionId = `eval-judge-${EVAL_NAME}-${attempt}-${Date.now()}`;
			try {
				await client.createSession(judgeSessionId, {
					agent: JUDGE,
					model: JUDGE_MODEL,
					...(JUDGE_VARIANT ? { variant: JUDGE_VARIANT } : {}),
					permissionMode: "bypass",
				});

				verdictRaw = await collectTurnText(client, judgeSessionId, retryInput);
			} catch (err) {
				judgeError = err as Error;
				try { await client.terminateSession(judgeSessionId); } catch {}
				break;
			}

			try { await client.terminateSession(judgeSessionId); } catch {}

			const parsed = parseVerdict(verdictRaw);
			if (parsed.verdict) {
				verdict = parsed.verdict;
				break;
			}

			lastValidationErrors = parsed.errors;
			if (attempt < MAX_JUDGE_ATTEMPTS) {
				console.log(`judge attempt ${attempt} invalid JSON, retrying`);
			}
		}

		// 5) Kill the server
		killServer(serverProc);
		if (serverLogs) serverLogs.closeLogs();

		if (judgeError) {
			const duration = Math.round((Date.now() - start) / 1000);
			console.log(`${fmt.red("ERROR")} (judge, ${duration}s)  ${judgeError.message.slice(0, 200)}`);
			console.log(`${fmt.bold("results:")} ${RESULTS_DIR}`);
			process.exit(1);
		}

		await writeFile(join(resultDir, "verdict.json"), verdict ? JSON.stringify(verdict, null, 2) : verdictRaw);

		// 6) Record result
		const duration = Math.round((Date.now() - start) / 1000);

		const meta = {
			agent: AGENT,
			model: MODEL,
			variant: VARIANT || undefined,
			judge: JUDGE,
			judge_model: JUDGE_MODEL,
			judge_variant: JUDGE_VARIANT || undefined,
			eval: EVAL_NAME,
			skills: skillIds,
			duration_s: duration,
			tmp_dir: tmpDir,
			has_friction_log: !!frictionLog,
			timestamp: new Date().toISOString(),
		};
		await writeFile(join(resultDir, "meta.json"), JSON.stringify(meta, null, 2));

		let result: Result = "ERROR";
		let summary = "";
		if (verdict && verdict.pass === true) {
			result = "PASS";
		} else if (verdict) {
			result = "FAIL";
			summary = verdict.summary;
		} else {
			result = "ERROR";
			summary = "Invalid judge verdict JSON";
		}

		if (result === "PASS") {
			console.log(`\n${fmt.green("PASS")} (${duration}s)`);
			if (!KEEP_TMP) {
				try {
					await rm(tmpDir, { recursive: true, force: true });
				} catch {
					// Non-fatal
				}
			}
			console.log(`${fmt.bold("results:")} ${RESULTS_DIR}`);
			process.exit(0);
		}

		console.log(`\n${result === "FAIL" ? fmt.red("FAIL") : fmt.red("ERROR")} (${duration}s)${summary ? `  ${summary}` : ""}`);
		console.log(`Project kept at: ${tmpDir}`);
		console.log(`${fmt.bold("results:")} ${RESULTS_DIR}`);
		process.exit(1);
	} finally {
		await client.dispose();
	}
}

(async () => {
	await main();
})().catch((err) => {
	console.error(err);
	process.exit(1);
});
