import type { Story } from "@ladle/react";
import { SOFTWARE } from "./fixtures";
import { SoftwareTab } from "./software-tab";
import { AgentOsStoryFrame } from "./story-frame";

export const Default: Story = () => (
	<AgentOsStoryFrame>
		<SoftwareTab software={SOFTWARE} />
	</AgentOsStoryFrame>
);

export const Empty: Story = () => (
	<AgentOsStoryFrame>
		<SoftwareTab software={[]} />
	</AgentOsStoryFrame>
);
