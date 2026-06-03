import type { Story } from "@ladle/react";
import { INVOCATIONS, TOOLKITS } from "./fixtures";
import { AgentOsStoryFrame } from "./story-frame";
import { ToolsTab } from "./tools-tab";

export const Default: Story = () => (
	<AgentOsStoryFrame>
		<ToolsTab toolkits={TOOLKITS} invocations={INVOCATIONS} />
	</AgentOsStoryFrame>
);

export const NoToolkits: Story = () => (
	<AgentOsStoryFrame>
		<ToolsTab toolkits={[]} invocations={[]} />
	</AgentOsStoryFrame>
);
