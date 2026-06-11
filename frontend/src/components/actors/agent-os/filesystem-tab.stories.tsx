import type { Story } from "@ladle/react";
import { FilesystemTab } from "./filesystem-tab";
import { FILE_CONTENTS, FILESYSTEM } from "./fixtures";
import { AgentOsStoryFrame } from "./story-frame";

const noop = () => {};

export const JsonFile: Story = () => (
	<AgentOsStoryFrame>
		<FilesystemTab
			tree={FILESYSTEM}
			selectedPath="/home/user/weather.json"
			content={FILE_CONTENTS["/home/user/weather.json"]}
			onSelect={noop}
		/>
	</AgentOsStoryFrame>
);

export const PythonFile: Story = () => (
	<AgentOsStoryFrame>
		<FilesystemTab
			tree={FILESYSTEM}
			selectedPath="/home/user/scratch.py"
			content={FILE_CONTENTS["/home/user/scratch.py"]}
			onSelect={noop}
		/>
	</AgentOsStoryFrame>
);

export const BinaryFile: Story = () => (
	<AgentOsStoryFrame>
		<FilesystemTab
			tree={FILESYSTEM}
			selectedPath="/mnt/sandbox/playwright-trace.zip"
			content={FILE_CONTENTS["/mnt/sandbox/playwright-trace.zip"]}
			onSelect={noop}
		/>
	</AgentOsStoryFrame>
);

export const NoSelection: Story = () => (
	<AgentOsStoryFrame>
		<FilesystemTab
			tree={FILESYSTEM}
			selectedPath={null}
			content={null}
			onSelect={noop}
		/>
	</AgentOsStoryFrame>
);
