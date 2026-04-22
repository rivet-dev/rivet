import {
	faCircleUser,
	faGoogle,
	faPlus,
	faShield,
	Icon,
} from "@rivet-gg/icons";
import { useState } from "react";
import { Button, cn } from "@/components";

type ProfileTab = "profile" | "security";

export function ProfileContent() {
	const [tab, setTab] = useState<ProfileTab>("profile");

	return (
		<div className="flex gap-6 pb-10">
			<aside className="w-48 shrink-0">
				<nav className="flex flex-col gap-0.5 sticky top-0">
					<SidebarItem
						icon={faCircleUser}
						label="Profile"
						active={tab === "profile"}
						onClick={() => setTab("profile")}
					/>
					<SidebarItem
						icon={faShield}
						label="Security"
						active={tab === "security"}
						onClick={() => setTab("security")}
					/>
				</nav>
			</aside>
			<div className="flex-1 min-w-0 space-y-6">
				{tab === "profile" ? <ProfileSection /> : <SecuritySection />}
			</div>
		</div>
	);
}

function SidebarItem({
	icon,
	label,
	active,
	onClick,
}: {
	icon: typeof faCircleUser;
	label: string;
	active: boolean;
	onClick: () => void;
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			className={cn(
				"flex items-center gap-2 rounded-md px-2.5 py-1.5 text-sm text-left transition-colors",
				active
					? "bg-accent text-foreground"
					: "text-muted-foreground hover:bg-accent/50 hover:text-foreground",
			)}
		>
			<Icon icon={icon} className="w-3.5 shrink-0" />
			<span>{label}</span>
		</button>
	);
}

function Card({
	title,
	description,
	action,
	children,
}: {
	title: string;
	description?: string;
	action?: React.ReactNode;
	children: React.ReactNode;
}) {
	return (
		<div className="rounded-xl border dark:border-white/10 bg-card overflow-hidden">
			<div className="flex items-start justify-between gap-4 px-6 pt-5 pb-4">
				<div>
					<h3 className="text-base font-semibold text-foreground">
						{title}
					</h3>
					{description ? (
						<p className="mt-0.5 text-xs text-muted-foreground">
							{description}
						</p>
					) : null}
				</div>
				{action ? <div className="shrink-0">{action}</div> : null}
			</div>
			<div className="border-t dark:border-white/10">{children}</div>
		</div>
	);
}

function Field({
	label,
	children,
	last,
}: {
	label: string;
	children: React.ReactNode;
	last?: boolean;
}) {
	return (
		<div
			className={cn(
				"grid grid-cols-[160px_1fr] items-center gap-4 px-6 py-3.5 text-sm",
				!last && "border-b dark:border-white/10",
			)}
		>
			<div className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
				{label}
			</div>
			<div className="min-w-0">{children}</div>
		</div>
	);
}

function ProfileSection() {
	return (
		<Card
			title="Profile details"
			description="Your personal information and connected identities."
		>
			<Field label="Profile">
				<div className="flex items-center justify-between gap-3">
					<div className="flex items-center gap-3 min-w-0">
						<div className="flex size-8 items-center justify-center rounded-full bg-blue-500 text-xs font-medium text-white shrink-0">
							NK
						</div>
						<span className="text-sm text-foreground truncate">
							Nicholas Kissel
						</span>
					</div>
					<Button variant="outline" size="sm">
						Update
					</Button>
				</div>
			</Field>
			<Field label="Email">
				<div className="space-y-2">
					<div className="flex items-center gap-2 min-w-0">
						<span className="text-sm text-foreground truncate">
							nicholas@rivet.dev
						</span>
						<span className="inline-flex shrink-0 items-center rounded-full border border-border bg-muted px-1.5 py-px text-[10px] font-medium text-muted-foreground">
							Primary
						</span>
					</div>
					<button
						type="button"
						className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors"
					>
						<Icon icon={faPlus} className="w-3" />
						Add email address
					</button>
				</div>
			</Field>
			<Field label="Connected accounts" last>
				<div className="flex items-center gap-2 min-w-0">
					<Icon
						icon={faGoogle}
						className="w-3.5 text-muted-foreground shrink-0"
					/>
					<span className="text-sm text-foreground">Google</span>
					<span className="text-muted-foreground/50 text-sm">·</span>
					<span className="text-sm text-muted-foreground truncate">
						nicholas@rivet.dev
					</span>
				</div>
			</Field>
		</Card>
	);
}

function SecuritySection() {
	return (
		<Card
			title="Security"
			description="Manage your password and verification methods."
		>
			<Field label="Password">
				<div className="flex items-center justify-between gap-3">
					<span className="text-sm text-muted-foreground">
						Not set
					</span>
					<Button variant="outline" size="sm">
						Set password
					</Button>
				</div>
			</Field>
			<Field label="Two-step" last>
				<div className="flex items-center justify-between gap-3">
					<span className="text-sm text-muted-foreground">
						Add an extra layer of security to your account.
					</span>
					<Button variant="outline" size="sm">
						Add method
					</Button>
				</div>
			</Field>
		</Card>
	);
}
