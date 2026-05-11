import type { Story } from "@ladle/react";
import { useState } from "react";
import "../../../.ladle/ladle.css";
import { TooltipProvider } from "@/components";
import { IconPicker, IconRenderer } from "./icon-picker";

function Frame({ children }: { children: React.ReactNode }) {
	return (
		<TooltipProvider>
			<div className="bg-background min-h-screen p-12">
				<div className="max-w-md mx-auto flex flex-col gap-4">
					{children}
				</div>
			</div>
		</TooltipProvider>
	);
}

export const Empty: Story = () => {
	const [icon, setIcon] = useState<string | null>(null);
	return (
		<Frame>
			<div className="flex items-center gap-3">
				<IconPicker value={icon} onChange={setIcon} />
				<span className="text-sm text-muted-foreground">
					Selected: {icon ?? "none"}
				</span>
			</div>
		</Frame>
	);
};

export const Preselected: Story = () => {
	const [icon, setIcon] = useState<string | null>("rocket");
	return (
		<Frame>
			<div className="flex items-center gap-3">
				<IconPicker value={icon} onChange={setIcon} />
				<span className="text-sm text-muted-foreground">
					Selected: {icon ?? "none"}
				</span>
			</div>
		</Frame>
	);
};

export const InsideProviderRow: Story = () => {
	const [icon, setIcon] = useState<string | null>("server");
	const [name, setName] = useState("my custom cluster");
	return (
		<Frame>
			<div className="border rounded-md p-4 flex flex-col gap-4">
				<div className="text-sm font-medium">Custom provider</div>
				<div className="flex items-center gap-2">
					<IconPicker value={icon} onChange={setIcon} />
					<input
						value={name}
						onChange={(e) => setName(e.target.value)}
						className="h-9 flex-1 rounded-md border bg-background px-3 text-sm"
						maxLength={32}
					/>
				</div>
				<div className="flex items-center gap-2 text-sm">
					Preview:
					<span className="inline-flex items-center gap-2 rounded bg-muted px-2 py-1">
						<IconRenderer name={icon} className="size-3.5" />
						{name || "Custom"}
					</span>
				</div>
			</div>
		</Frame>
	);
};

export const UnknownIconName: Story = () => {
	const [icon, setIcon] = useState<string | null>("not-a-real-icon");
	return (
		<Frame>
			<div className="flex items-center gap-3">
				<IconPicker value={icon} onChange={setIcon} />
				<span className="text-sm text-muted-foreground">
					Value "{icon}" doesn't resolve — trigger falls back to the fa
					question mark.
				</span>
			</div>
		</Frame>
	);
};
