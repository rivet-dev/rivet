import type { Story } from "@ladle/react";
import { useState } from "react";
import "../../../.ladle/ladle.css";
import {
	Checkbox,
	Combobox,
	Input,
	Label,
	MultiSelectFormField,
	RadioGroup,
	RadioGroupItem,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
	Slider,
	Switch,
	Textarea,
	TooltipProvider,
} from "@/components";

// This gallery exists to compare every input-like control side by side at the
// same width. Background tint, border color, and height must match across
// Input, Textarea, Select, and Combobox. The Combobox trigger previously used
// an opaque `bg-background` while the others use `bg-foreground/[0.02]`, which
// made it render visibly brighter than its neighbors. Keep this story as the
// visual regression check for that consistency.

const REGION_OPTIONS = [
	{ label: "Global", value: "global" },
	{ label: "US East (us-east-1)", value: "us-east-1" },
	{ label: "US West (us-west-2)", value: "us-west-2" },
	{ label: "EU Central (eu-central-1)", value: "eu-central-1" },
	{ label: "Asia Pacific (ap-southeast-1)", value: "ap-southeast-1" },
];

function Frame({ children }: { children: React.ReactNode }) {
	return (
		<TooltipProvider>
			<div className="bg-background min-h-screen p-12">
				<div className="max-w-md space-y-8">{children}</div>
			</div>
		</TooltipProvider>
	);
}

function Field({
	label,
	children,
}: {
	label: string;
	children: React.ReactNode;
}) {
	return (
		<div className="space-y-2">
			<Label>{label}</Label>
			{children}
		</div>
	);
}

function ComboboxField({ placeholder }: { placeholder: string }) {
	const [value, setValue] = useState<string | undefined>(undefined);
	return (
		<Combobox
			placeholder={placeholder}
			options={REGION_OPTIONS}
			value={value}
			onValueChange={setValue}
			className="w-full"
		/>
	);
}

function MultiSelectField({ placeholder }: { placeholder: string }) {
	const [, setValue] = useState<string[]>([]);
	return (
		<MultiSelectFormField
			placeholder={placeholder}
			options={REGION_OPTIONS}
			onValueChange={setValue}
			className="w-full"
		/>
	);
}

// All text-entry / picker controls stacked at one width so any background or
// border mismatch is obvious. This is the primary consistency check.
export const Consistency: Story = () => (
	<Frame>
		<Field label="Input">
			<Input placeholder="https://your-deployment.com" />
		</Field>
		<Field label="Input (filled)">
			<Input defaultValue="https://your-deployment.com/api/rivet" />
		</Field>
		<Field label="Textarea">
			<Textarea placeholder="Describe your deployment..." />
		</Field>
		<Field label="Select">
			<Select>
				<SelectTrigger>
					<SelectValue placeholder="Choose a region..." />
				</SelectTrigger>
				<SelectContent>
					{REGION_OPTIONS.map((o) => (
						<SelectItem key={o.value} value={o.value}>
							{o.label}
						</SelectItem>
					))}
				</SelectContent>
			</Select>
		</Field>
		<Field label="Combobox">
			<ComboboxField placeholder="Choose a region..." />
		</Field>
		<Field label="Multi-select">
			<MultiSelectField placeholder="Choose regions..." />
		</Field>
	</Frame>
);

export const Disabled: Story = () => (
	<Frame>
		<Field label="Input">
			<Input placeholder="https://your-deployment.com" disabled />
		</Field>
		<Field label="Textarea">
			<Textarea placeholder="Describe your deployment..." disabled />
		</Field>
		<Field label="Select">
			<Select disabled>
				<SelectTrigger>
					<SelectValue placeholder="Choose a region..." />
				</SelectTrigger>
			</Select>
		</Field>
	</Frame>
);

// Toggle-style controls. Grouped separately because their sizing rules differ
// from the full-width text controls above.
export const Toggles: Story = () => (
	<Frame>
		<div className="flex items-center gap-2">
			<Checkbox id="cb-1" defaultChecked />
			<Label htmlFor="cb-1">Enable multi-region routing</Label>
		</div>
		<div className="flex items-center gap-2">
			<Checkbox id="cb-2" />
			<Label htmlFor="cb-2">Require authentication</Label>
		</div>
		<div className="flex items-center gap-2">
			<Switch id="sw-1" defaultChecked />
			<Label htmlFor="sw-1">Serverless mode</Label>
		</div>
		<Field label="Datacenter">
			<RadioGroup defaultValue="global" className="space-y-2">
				{REGION_OPTIONS.slice(0, 3).map((o) => (
					<div key={o.value} className="flex items-center gap-2">
						<RadioGroupItem id={`rg-${o.value}`} value={o.value} />
						<Label htmlFor={`rg-${o.value}`}>{o.label}</Label>
					</div>
				))}
			</RadioGroup>
		</Field>
		<Field label="Request lifespan">
			<Slider defaultValue={[900]} min={0} max={3600} step={30} />
		</Field>
	</Frame>
);
