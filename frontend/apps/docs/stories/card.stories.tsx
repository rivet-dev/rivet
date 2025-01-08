import type { Meta, StoryObj } from "@storybook/react";

import {
	Button,
	Card as CardCmp,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
	Flex,
	InputOTP,
	InputOTPGroup,
	InputOTPSlot,
	Label,
} from "@rivet-gg/components";

const meta = {
	title: "Card",
	component: CardCmp,
	args: {},
	argTypes: {},
} satisfies Meta<typeof CardCmp>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Card: Story = {
	render: (props) => (
		<CardCmp {...props}>
			<CardHeader>
				<CardTitle>Welcome to Rivet!</CardTitle>
				<CardDescription>
					Check your email for a verification code from hello@rivet.gg
					and paste it into the area below.
				</CardDescription>
			</CardHeader>
			<CardContent>
				<Flex gap="2" direction="col">
					<Label>Code</Label>
					<InputOTP maxLength={8}>
						<InputOTPGroup>
							<InputOTPSlot index={0} />
							<InputOTPSlot index={1} />
							<InputOTPSlot index={2} />
							<InputOTPSlot index={3} />
							<InputOTPSlot index={4} />
							<InputOTPSlot index={5} />
							<InputOTPSlot index={6} />
							<InputOTPSlot index={7} />
						</InputOTPGroup>
					</InputOTP>
				</Flex>
			</CardContent>
			<CardFooter>
				<Flex gap="4">
					<Button type="button" variant="secondary">
						Cancel
					</Button>
					<Button>Continue</Button>
				</Flex>
			</CardFooter>
		</CardCmp>
	),
};

export const WithoutActions: Story = {
	render: (props) => (
		<CardCmp {...props}>
			<CardHeader>
				<CardTitle>Welcome to Rivet!</CardTitle>
			</CardHeader>
			<CardContent>
				Lorem ipsum dolor sit amet consectetur adipisicing elit. Alias
				accusantium placeat atque perferendis deleniti ut animi quos
				tempore maiores dolorum. Nihil, deserunt? Autem laudantium
				cupiditate ipsum! Non eius at dolorem?
			</CardContent>
		</CardCmp>
	),
};
