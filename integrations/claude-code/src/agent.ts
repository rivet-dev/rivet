import {
	type ActionContext,
	type ActorDefinition,
	actor,
	event,
	type Type,
	UserError,
} from "rivetkit";
import { runClaudeCode } from "./run.js";

const DEFAULT_TURN_TIMEOUT_MS = 10 * 60_000;

export interface ClaudeCodeAgentOptions {
	systemPrompt: string;
	tools?: readonly string[] | undefined;
	model?: string | undefined;
	cwd?: string | undefined;
	turnTimeoutMs?: number | undefined;
	claudeCodePath?: string | undefined;
}

export interface ClaudeCodeTurn {
	prompt: string;
	response: string;
	sessionId: string;
	costUsd: number;
	durationMs: number;
	completedAt: number;
}

export interface ClaudeCodeAgentState {
	sessionId: string | null;
	turns: ClaudeCodeTurn[];
}

interface ClaudeCodeAgentVars {
	turnInProgress: boolean;
}

type ClaudeCodeAgentEvents = {
	turn: Type<ClaudeCodeTurn>;
};

type ClaudeCodeAgentContext = ActionContext<
	ClaudeCodeAgentState,
	unknown,
	unknown,
	ClaudeCodeAgentVars,
	unknown,
	undefined,
	ClaudeCodeAgentEvents,
	Record<never, never>
>;

type ClaudeCodeAgentActions = {
	prompt: (c: ClaudeCodeAgentContext, prompt: string) => Promise<ClaudeCodeTurn>;
	getTurns: (c: ClaudeCodeAgentContext) => ClaudeCodeTurn[];
};

export type ClaudeCodeAgentDefinition = ActorDefinition<
	ClaudeCodeAgentState,
	unknown,
	unknown,
	ClaudeCodeAgentVars,
	unknown,
	undefined,
	ClaudeCodeAgentEvents,
	Record<never, never>,
	ClaudeCodeAgentActions
>;

const turnEvent: Type<ClaudeCodeTurn> = event<ClaudeCodeTurn>();

export function claudeCodeAgent(options: ClaudeCodeAgentOptions): ClaudeCodeAgentDefinition {
	return actor({
		createState: (): ClaudeCodeAgentState => ({
			sessionId: null,
			turns: [],
		}),
		createVars: (): ClaudeCodeAgentVars => ({
			turnInProgress: false,
		}),
		events: {
			turn: turnEvent,
		},
		options: {
			actionTimeout: options.turnTimeoutMs ?? DEFAULT_TURN_TIMEOUT_MS,
		},
		actions: {
			prompt: async (c, prompt: string): Promise<ClaudeCodeTurn> => {
				if (c.vars.turnInProgress) {
					throw new UserError("This agent is already working on a prompt.", {
						code: "claude_code_turn_in_progress",
					});
				}

				c.vars.turnInProgress = true;
				try {
					const run = await runClaudeCode({
						prompt,
						systemPrompt: options.systemPrompt,
						tools: options.tools,
						model: options.model,
						cwd: options.cwd,
						claudeCodePath: options.claudeCodePath,
						resumeSessionId: c.state.sessionId ?? undefined,
						persistSession: true,
						signal: c.abortSignal,
					});

					const turn: ClaudeCodeTurn = {
						prompt,
						response: run.output,
						sessionId: run.sessionId,
						costUsd: run.costUsd,
						durationMs: run.durationMs,
						completedAt: Date.now(),
					};
					c.state.sessionId = run.sessionId;
					c.state.turns.push(turn);
					c.broadcast("turn", turn);
					return turn;
				} finally {
					c.vars.turnInProgress = false;
				}
			},
			getTurns: (c) => c.state.turns,
		},
	});
}
