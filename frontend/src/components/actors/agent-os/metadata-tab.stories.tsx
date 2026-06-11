import type { Story } from "@ladle/react";
import { METADATA } from "./fixtures";
import { MetadataTab } from "./metadata-tab";
import { AgentOsStoryFrame } from "./story-frame";

export const Default: Story = () => (
	<AgentOsStoryFrame>
		<MetadataTab metadata={METADATA} />
	</AgentOsStoryFrame>
);

export const SleptWithMoreSessions: Story = () => (
	<AgentOsStoryFrame>
		<MetadataTab
			metadata={{
				...METADATA,
				lastSleepAt: Date.parse("2026-05-30T09:02:00Z"),
				sessionCount: 12,
				activeSessionCount: 0,
			}}
		/>
	</AgentOsStoryFrame>
);
