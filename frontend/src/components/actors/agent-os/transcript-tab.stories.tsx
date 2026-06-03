import type { Story } from "@ladle/react";
import { SESSIONS, TRANSCRIPTS } from "./fixtures";
import { AgentOsStoryFrame } from "./story-frame";
import { TranscriptTab } from "./transcript-tab";

export const RunningSession: Story = () => (
	<AgentOsStoryFrame>
		<TranscriptTab
			session={SESSIONS[0]}
			events={TRANSCRIPTS[SESSIONS[0].sessionId] ?? []}
		/>
	</AgentOsStoryFrame>
);

export const FailedShellRun: Story = () => (
	<AgentOsStoryFrame>
		<TranscriptTab
			session={SESSIONS[1]}
			events={TRANSCRIPTS[SESSIONS[1].sessionId] ?? []}
		/>
	</AgentOsStoryFrame>
);

export const ToolError: Story = () => (
	<AgentOsStoryFrame>
		<TranscriptTab
			session={SESSIONS[3]}
			events={TRANSCRIPTS[SESSIONS[3].sessionId] ?? []}
		/>
	</AgentOsStoryFrame>
);

export const EmptySession: Story = () => (
	<AgentOsStoryFrame>
		<TranscriptTab session={SESSIONS[2]} events={[]} />
	</AgentOsStoryFrame>
);

export const NoSelection: Story = () => (
	<AgentOsStoryFrame>
		<TranscriptTab session={null} events={[]} />
	</AgentOsStoryFrame>
);
