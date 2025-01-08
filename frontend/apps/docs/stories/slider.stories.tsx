import type { Meta, StoryObj } from "@storybook/react";

import { Slider as SliderCmp } from "@rivet-gg/components";

const meta = {
	title: "Slider",
	component: SliderCmp,
	args: {},
	argTypes: {
		min: {
			control: { type: "number" },
		},
		max: {
			control: { type: "number" },
		},
		step: {
			control: { type: "number" },
		},
		minStepsBetweenThumbs: {
			control: { type: "number" },
		},
	},
} satisfies Meta<typeof SliderCmp>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Slider: Story = {
	render: (props) => <SliderCmp {...props} />,
};

export const TwoThumbs: Story = {
	render: (props) => <SliderCmp {...props} defaultValue={[0, 100]} />,
};
