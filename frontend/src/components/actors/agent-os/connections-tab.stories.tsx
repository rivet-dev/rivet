import type { Story } from "@ladle/react";
import { ConnectionsTab } from "./connections-tab";
import { CONNECTIONS } from "./fixtures";
import { AgentOsStoryFrame } from "./story-frame";

export const Default: Story = () => (
	<AgentOsStoryFrame>
		<ConnectionsTab connections={CONNECTIONS} />
	</AgentOsStoryFrame>
);

export const Empty: Story = () => (
	<AgentOsStoryFrame>
		<ConnectionsTab connections={[]} />
	</AgentOsStoryFrame>
);
