import type { Meta, StoryObj } from "@storybook/react";

import {
	Avatar as AvatarCmp,
	AvatarFallback,
	AvatarImage,
} from "@rivet-gg/components";

const meta = {
	title: "Avatar",
	component: AvatarCmp,
	args: {},
	argTypes: {},
} satisfies Meta<typeof AvatarCmp>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Avatar: Story = {
	render: (props) => (
		<AvatarCmp {...props}>
			<AvatarImage src="https://assets2.rivet.gg/avatars/avatar-3.png" />
			<AvatarFallback>JD</AvatarFallback>
		</AvatarCmp>
	),
};
