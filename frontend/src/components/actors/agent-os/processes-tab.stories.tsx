import type { Story } from "@ladle/react";
import { DEFAULT_PID, PROCESSES } from "./fixtures";
import { ProcessesTab } from "./processes-tab";
import { AgentOsStoryFrame } from "./story-frame";

export const Default: Story = () => (
	<AgentOsStoryFrame>
		<ProcessesTab processes={PROCESSES} defaultPid={DEFAULT_PID} />
	</AgentOsStoryFrame>
);

export const Empty: Story = () => (
	<AgentOsStoryFrame>
		<ProcessesTab processes={[]} />
	</AgentOsStoryFrame>
);
