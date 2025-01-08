import type { Meta, StoryObj } from "@storybook/react";

import { Badge as BadgeCmp } from "@rivet-gg/components";

const meta = {
	title: "Badge",
	component: BadgeCmp,
	args: {
		variant: "default",
	},
	argTypes: {
		variant: {
			control: "select",
			options: ["default", "destructive", "outline", "secondary"],
		},
	},
} satisfies Meta<typeof BadgeCmp>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Badge: Story = {
	render: (props) => <BadgeCmp {...props}>Badge</BadgeCmp>,
};
