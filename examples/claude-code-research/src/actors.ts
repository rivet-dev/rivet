import { claudeCodeAgent, runClaudeCode } from "@rivet-dev/claude-code";
import { type ActorContextOf, actor, event, setup, UserError } from "rivetkit";
import { z } from "zod";

const RESEARCH_MODEL = process.env.RESEARCH_MODEL;

const PLANNER_SYSTEM_PROMPT =
	"You plan research. Split the question you are given into 2 to 4 independent sub-questions that together answer it.";
const RESEARCHER_SYSTEM_PROMPT =
	"You are a research agent. Investigate the question you are given with web search and web fetch. Reply with a concise, factual summary of what you found and cite the source URLs.";
const SYNTHESIZER_SYSTEM_PROMPT =
	"You write research reports. Combine the findings you are given into a clear answer to the original question and keep the source URLs.";

const researchPlanSchema = z.object({
	subQuestions: z.array(z.string()).min(2).max(4),
});

export type ResearchStatus = "idle" | "planning" | "investigating" | "synthesizing" | "done" | "failed";

export interface ResearchProgress {
	status: ResearchStatus;
	message: string;
}

export interface Finding {
	subQuestion: string;
	answer: string;
	costUsd: number;
}

export interface ResearchState {
	question: string | null;
	status: ResearchStatus;
	findings: Finding[];
	report: string | null;
	error: string | null;
	costUsd: number;
}

export const researcher = claudeCodeAgent({
	systemPrompt: RESEARCHER_SYSTEM_PROMPT,
	tools: ["WebSearch", "WebFetch"],
	model: RESEARCH_MODEL,
});

export const researchCoordinator = actor({
	createState: (): ResearchState => ({
		question: null,
		status: "idle",
		findings: [],
		report: null,
		error: null,
		costUsd: 0,
	}),
	events: {
		progress: event<ResearchProgress>(),
	},
	actions: {
		start: (c, question: string) => {
			if (c.state.status !== "idle") {
				throw new UserError("This research has already started.");
			}
			c.state.question = question;
			void c.keepAwake(research(c, question));
			return { status: c.state.status };
		},
		getResult: (c): ResearchState => ({ ...c.state }),
	},
});

type CoordinatorContext = ActorContextOf<typeof researchCoordinator>;

async function research(c: CoordinatorContext, question: string): Promise<void> {
	try {
		const subQuestions = await planSubQuestions(c, question);
		reportProgress(c, "investigating", `Investigating ${subQuestions.length} sub-questions in parallel`);
		await Promise.all(subQuestions.map((subQuestion, index) => investigate(c, subQuestion, index)));
		c.state.report = await writeReport(c, question);
		reportProgress(c, "done", "Report ready");
	} catch (error) {
		c.state.error = error instanceof Error ? error.message : String(error);
		reportProgress(c, "failed", c.state.error);
	}
}

async function planSubQuestions(c: CoordinatorContext, question: string): Promise<string[]> {
	reportProgress(c, "planning", "Planning sub-questions");
	const plan = await runClaudeCode(
		{ prompt: question, systemPrompt: PLANNER_SYSTEM_PROMPT, model: RESEARCH_MODEL },
		researchPlanSchema,
	);
	c.state.costUsd += plan.costUsd;
	return plan.output.subQuestions;
}

async function investigate(c: CoordinatorContext, subQuestion: string, index: number): Promise<void> {
	const researcherForQuestion = c.client<typeof registry>().researcher.getOrCreate([...c.key, `${index}`]);
	const turn = await researcherForQuestion.prompt(subQuestion);
	c.state.findings.push({ subQuestion, answer: turn.response, costUsd: turn.costUsd });
	c.state.costUsd += turn.costUsd;
	reportProgress(c, "investigating", `Finished: ${subQuestion}`);
}

async function writeReport(c: CoordinatorContext, question: string): Promise<string> {
	reportProgress(c, "synthesizing", "Writing the report");
	const report = await runClaudeCode({
		prompt: synthesisPrompt(question, c.state.findings),
		systemPrompt: SYNTHESIZER_SYSTEM_PROMPT,
		model: RESEARCH_MODEL,
	});
	c.state.costUsd += report.costUsd;
	return report.output;
}

function reportProgress(c: CoordinatorContext, status: ResearchStatus, message: string): void {
	c.state.status = status;
	c.broadcast("progress", { status, message });
}

function synthesisPrompt(question: string, findings: Finding[]): string {
	const findingSections = findings.map((finding) => `## ${finding.subQuestion}\n\n${finding.answer}`);
	return [`# Question\n\n${question}`, "# Findings", ...findingSections].join("\n\n");
}

export const registry = setup({
	use: { researchCoordinator, researcher },
});
