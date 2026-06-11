import type { Story } from "@ladle/react";
import { MOUNTS } from "./fixtures";
import { MountsTab } from "./mounts-tab";
import { AgentOsStoryFrame } from "./story-frame";

export const Default: Story = () => (
	<AgentOsStoryFrame>
		<MountsTab mounts={MOUNTS} />
	</AgentOsStoryFrame>
);

export const AllDegraded: Story = () => (
	<AgentOsStoryFrame>
		<MountsTab
			mounts={MOUNTS.map((mount) => ({
				...mount,
				status: "degraded" as const,
			}))}
		/>
	</AgentOsStoryFrame>
);

export const Empty: Story = () => (
	<AgentOsStoryFrame>
		<MountsTab mounts={[]} />
	</AgentOsStoryFrame>
);
