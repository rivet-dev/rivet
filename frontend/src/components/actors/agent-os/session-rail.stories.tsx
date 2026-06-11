import type { Story } from "@ladle/react";
import { SESSIONS } from "./fixtures";
import { SessionRail } from "./session-rail";
import { AgentOsStoryFrame } from "./story-frame";

const noop = () => {};

export const Default: Story = () => (
	<AgentOsStoryFrame>
		<div className="flex h-full">
			<SessionRail
				sessions={SESSIONS}
				selectedSessionId={SESSIONS[0].sessionId}
				onSelectSession={noop}
				agentName="pi-coding-agent"
			/>
		</div>
	</AgentOsStoryFrame>
);

export const Empty: Story = () => (
	<AgentOsStoryFrame>
		<div className="flex h-full">
			<SessionRail
				sessions={[]}
				selectedSessionId={null}
				onSelectSession={noop}
				agentName="pi-coding-agent"
			/>
		</div>
	</AgentOsStoryFrame>
);
