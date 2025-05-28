import { ActionContextOf, actor } from "actor-core";
import { LinearClient } from "@linear/sdk";
import { actorClient } from "./app";
import { WebhookIssue, WebhookComment } from "../linear-types";
import { CoreMessage, generateText } from "ai";
import { anthropic } from "@ai-sdk/anthropic";

interface IssueAgentState {
	messages: CoreMessage[];
}

export const issueAgent = actor({
	state: {
		messages: [],
	} as IssueAgentState,
	actions: {
		issueMention: async (c, appUserId: string, issue: WebhookIssue) => {
			// Do nothing
		},
		issueEmojiReaction: async (
			c,
			appUserId: string,
			issue: WebhookIssue,
			emoji: string,
		) => {
			// Do nothing
		},
		issueCommentMention: async (
			c,
			appUserId: string,
			issue: WebhookIssue,
			comment: WebhookComment,
		) => {
			c.log.info("mentioned in comment", {
				issue: issue.id,
				comment: comment.id,
			});

			const linearClient = await buildLinearClient(appUserId);

			c.log.info("acknowledging comment", {
				issue: issue.id,
				comment: comment.id,
			});
			await linearClient.createReaction({
				commentId: comment.id,
				emoji: "👀",
			});

			c.log.info("generating response to comment", {
				issue: issue.id,
				comment: comment.id,
			});
			const fetchedComment = await linearClient.comment({
				id: comment.id,
			});
			const response = await prompt(
				c,
				`The user mentioned me in a comment:\n\n\`\`\`\n${comment.body}\n\`\`\``,
			);
			await linearClient.createComment({
				issueId: issue.id,
				// Must use the top-most comment ID
				parentId: fetchedComment.parentId ?? comment.id,
				body: response,
			});
		},
		issueAssignedToYou: async (
			c,
			appUserId: string,
			issue: WebhookIssue,
		) => {
			c.log.info(`Issue assigned to app: ${issue.title} (${issue.id})`);
			const linearClient = await buildLinearClient(appUserId);

			// Set issue as started
			c.log.info("finding issue state", { issue: issue.id });
			const fetchedIssue = await linearClient.issue(issue.id);
			const state = await fetchedIssue.state;
			if (
				state &&
				state.type !== "started" &&
				state.type !== "completed" &&
				state.type !== "canceled"
			) {
				const states = await linearClient.workflowStates();
				const startedState = states.nodes.find(
					(s) => s.type === "started",
				);
				if (startedState) {
					c.log.info("updating issue state", {
						issue: issue.id,
						state: startedState.id,
					});
					await fetchedIssue.update({ stateId: startedState.id });
				} else {
					c.log.warn("could not find started state");
				}
			}

			// Generate response
			c.log.info("generating response to issue", { issue: issue.id });
			const response = await prompt(
				c,
				`I've been assigned to issue: "${issue.title}". The description is:\n\n\`\`\`\n${fetchedIssue.description}\n\`\`\``,
			);
			await linearClient.createComment({
				issueId: issue.id,
				body: response,
			});
		},
		issueCommentReaction: async (
			c,
			appUserId: string,
			issue: WebhookIssue,
			comment: WebhookComment,
			emoji: string,
		) => {
			// Do nothing
		},
		issueUnassignedFromYou: async (
			c,
			appUserId: string,
			issue: WebhookIssue,
		) => {
			const linearClient = await buildLinearClient(appUserId);

			c.log.info("responding to issue unassigned", { issue: issue.id });
			await linearClient.createComment({
				issueId: issue.id,
				body: "I've been unassigned from this issue.",
			});
		},
		issueNewComment: async (
			c,
			appUserId: string,
			issue: WebhookIssue,
			comment: WebhookComment,
		) => {
			// Do nothing
		},
		issueStatusChanged: async (
			c,
			appUserId: string,
			issue: WebhookIssue,
		) => {
			// Do nothing
		},
	},
});

async function buildLinearClient(appUserId: string): Promise<LinearClient> {
	const accessToken = await actorClient.linearAppUser
		.get(appUserId)
		.getAccessToken();
	return new LinearClient({ accessToken });
}

const SYSTEM_PROMPT = `
You are a code generation assistant for Linear. Your job is to:

1. Read issue descriptions and generate appropriate code solutions
2. Iterate on your code based on comments and feedback
3. Provide brief explanations of your implementation

When responding:
- Always provide the full requested code
- Do not exclude parts of the code, always include the full code
- Focus on delivering working code that meets requirements
- Keep explanations concise and relevant
- If no language is specified, use TypeScript

Your goal is to save developers time by providing ready-to-implement solutions.
`;

async function prompt(c: ActionContextOf<typeof issueAgent>, content: string) {
	c.log.debug("generating text", { messages: c.state.messages });

	c.state.messages.push({ role: "user", content });

	const { text, response } = await generateText({
		model: anthropic("claude-4-opus-20250514"),
		system: SYSTEM_PROMPT,
		messages: c.state.messages,
	});

	c.state.messages.push(...response.messages);

	return text;
}
