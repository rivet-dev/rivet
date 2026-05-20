import type { Story } from "@ladle/react";
import { faPlus } from "@rivet-gg/icons";
import { Icon } from "@rivet-gg/icons";
import "../../../.ladle/ladle.css";
import { Button } from "@/components";
import { SettingsCard } from "./settings-card";

/**
 * Stories cover the layout modes that decide whether the card chrome stays
 * consistent: padded free-form content, a divided row list, a header with a
 * right-aligned action, and a header-less row list. These are the shapes every
 * settings screen reaches for, so a regression in any of them misaligns content
 * across tabs.
 */
function Frame({ children }: { children: React.ReactNode }) {
	return (
		<div className="bg-background min-h-screen p-6">
			<div className="mx-auto max-w-2xl space-y-6">{children}</div>
		</div>
	);
}

function Row({ label, value, last }: { label: string; value: string; last?: boolean }) {
	return (
		<div
			className={
				"grid grid-cols-[160px_1fr] items-center gap-4 px-5 py-3.5 text-sm" +
				(last ? "" : " border-b border-foreground/10")
			}
		>
			<div className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
				{label}
			</div>
			<div className="text-foreground">{value}</div>
		</div>
	);
}

const PaddedCard = (
	<SettingsCard
		title="Free-form content"
		description="Default padded mode shares the card padding with the header."
	>
		<p className="text-sm text-muted-foreground">
			Anything can go here. The card supplies the border, radius,
			background, and a single consistent inner padding.
		</p>
	</SettingsCard>
);

const WithActionCard = (
	<SettingsCard
		title="Providers"
		description="Clouds connected to Rivet for running Rivet Actors."
		action={
			<Button
				variant="outline"
				size="sm"
				startIcon={<Icon icon={faPlus} className="size-3" />}
			>
				Add provider
			</Button>
		}
	>
		<div className="rounded-md border border-foreground/10 p-4 text-sm text-muted-foreground">
			Table or content slot.
		</div>
	</SettingsCard>
);

const DividedRowsCard = (
	<SettingsCard
		divided
		title="Profile details"
		description="Divided mode separates a header from a self-padded row list."
	>
		<Row label="Profile" value="Kacper" />
		<Row label="Email" value="kacper@rivet.gg" />
		<Row label="Connected accounts" value="Google" last />
	</SettingsCard>
);

const HeaderlessRowsCard = (
	<SettingsCard divided>
		<Row label="Members" value="1" />
		<Row label="Pending invites" value="0" last />
	</SettingsCard>
);

export const Padded: Story = () => <Frame>{PaddedCard}</Frame>;
export const WithAction: Story = () => <Frame>{WithActionCard}</Frame>;
export const DividedRows: Story = () => <Frame>{DividedRowsCard}</Frame>;
export const HeaderlessRows: Story = () => <Frame>{HeaderlessRowsCard}</Frame>;

export const Gallery: Story = () => (
	<Frame>
		{PaddedCard}
		{WithActionCard}
		{DividedRowsCard}
		{HeaderlessRowsCard}
	</Frame>
);
