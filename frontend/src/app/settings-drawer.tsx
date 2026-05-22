import * as DialogPrimitive from "@radix-ui/react-dialog";
import {
	faBuilding,
	faCircleUser,
	faClose,
	faCreditCard,
	faGear,
	faSliders,
	faSparkles,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import {
	useMatch,
	useMatchRoute,
	useNavigate,
	useSearch,
} from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { type ReactNode, useEffect, useState } from "react";
import { cn, VisuallyHidden } from "@/components";
import {
	useCloudNamespaceDataProvider,
	useCloudProjectDataProvider,
} from "@/components/actors/data-provider";
import { BillingUsageGauge } from "./billing/billing-usage-gauge";
import { BillingPanel } from "./settings-pages/billing-panel";
import {
	NamespaceAdvancedContent,
	NamespaceSettingsContent,
} from "./settings-pages/namespace-settings";
import { OrganizationPanel } from "./settings-pages/organization-panel";
import { ProfilePage } from "./settings-pages/profile-page";
import { ResourcePicker } from "./settings-pages/resource-picker";
import { WhatsNewPanel } from "./settings-pages/whats-new-panel";

export type SettingsTab =
	| "profile"
	| "settings"
	| "advanced"
	| "billing"
	| "organization"
	| "whats-new";

const NAV_SECTIONS: Array<{
	label: string;
	items: { key: SettingsTab; label: string; icon: IconProp }[];
}> = [
	{
		label: "Account",
		items: [{ key: "profile", label: "Account", icon: faCircleUser }],
	},
	{
		label: "Project",
		items: [
			{ key: "billing", label: "Billing", icon: faCreditCard },
		],
	},
	{
		label: "Namespace",
		items: [
			{ key: "settings", label: "Settings", icon: faGear },
			{ key: "advanced", label: "Advanced", icon: faSliders },
		],
	},
	{
		label: "Organization",
		items: [
			{ key: "organization", label: "Organization", icon: faBuilding },
		],
	},
	{
		label: "Other",
		items: [
			{ key: "whats-new", label: "What's new", icon: faSparkles },
		],
	},
];

const TAB_META: Record<
	SettingsTab,
	{ title: string; description?: string }
> = {
	profile: {
		title: "Account",
		description: "Manage your account info.",
	},
	billing: {
		title: "Billing",
		description:
			"Manage your project's billing information and view usage details.",
	},
	settings: {
		title: "Settings",
		description:
			"Connect your RivetKit application to Rivet Cloud. Use your cloud of choice to run Rivet Actors.",
	},
	advanced: {
		title: "Advanced",
		description:
			"Tokens, datacenter status, and other low-level controls for this namespace.",
	},
	organization: {
		title: "Organization",
		description: "Manage your organization and its members.",
	},
	"whats-new": {
		title: "What's new",
		description: "Recent changes and announcements.",
	},
};

interface SettingsDrawerProps {
	open: boolean;
	tab: SettingsTab;
	onOpenChange: (open: boolean) => void;
}

// TopBar is `h-11` with `mt-2` (8 + 44 = 52px). Add another 8px gap to match the
// content view's `my-2` so the drawer's top edge aligns with the card below.
const TOP_BAR_OUTER_HEIGHT = "60px";


export function SettingsDrawer({
	open,
	tab,
	onOpenChange,
}: SettingsDrawerProps) {
	const navigate = useNavigate();
	const matchRoute = useMatchRoute();
	const onNamespace = !!matchRoute({
		to: "/orgs/$organization/projects/$project/ns/$namespace",
		fuzzy: true,
	});
	const [activeTab, setActiveTab] = useState<SettingsTab>(tab);

	useEffect(() => {
		if (open) setActiveTab(tab);
	}, [open, tab]);

	const switchTab = (next: SettingsTab) => {
		setActiveTab(next);
		navigate({
			to: ".",
			search: (old) => ({
				...(old as Record<string, unknown>),
				settings: next,
			}),
		});
	};

	const onProject = !!matchRoute({
		to: "/orgs/$organization/projects/$project",
		fuzzy: true,
	});

	const meta = TAB_META[activeTab];
	const titleNode: ReactNode =
		activeTab === "settings" && onNamespace ? (
			<NamespaceSettingsTitle fallback={meta.title} />
		) : activeTab === "billing" && onProject ? (
			<ProjectBillingTitle fallback={meta.title} />
		) : (
			meta.title
		);

	return (
		<DialogPrimitive.Root
			open={open}
			onOpenChange={onOpenChange}
			modal={false}
		>
			<DialogPrimitive.Portal>
				<DialogPrimitive.Content
					className={cn(
						"fixed left-2 right-2 z-50 flex flex-col overflow-hidden",
						"bg-card border border-border rounded-lg",
						"focus:outline-none",
						"data-[state=open]:animate-in data-[state=closed]:animate-out",
						"data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0",
					)}
					style={{ top: TOP_BAR_OUTER_HEIGHT, bottom: "8px" }}
					onInteractOutside={(e) => e.preventDefault()}
					onPointerDownOutside={(e) => e.preventDefault()}
					onFocusOutside={(e) => e.preventDefault()}
				>
					<VisuallyHidden>
						<DialogPrimitive.Title>{meta.title}</DialogPrimitive.Title>
					</VisuallyHidden>

					<div className="flex h-full min-h-0">
						<aside className="w-44 shrink-0 border-r border-border p-3 overflow-y-auto">
							<nav className="flex flex-col gap-4">
								{NAV_SECTIONS.map((section) => (
									<div key={section.label}>
										<div className="px-2 pb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
											{section.label}
										</div>
										{section.items.map((item) => (
											<NavItem
												key={item.key}
												icon={item.icon}
												label={item.label}
												active={activeTab === item.key}
												onClick={() => switchTab(item.key)}
												trailing={
													item.key === "billing" &&
													activeTab !== "billing" ? (
														<NavBillingGauge />
													) : null
												}
											/>
										))}
									</div>
								))}
							</nav>
						</aside>
						<div className="flex-1 min-w-0 overflow-y-auto [scrollbar-gutter:stable]">
							<TabFrame
								title={titleNode}
								description={meta.description}
							>
								<TabContent tab={activeTab} />
							</TabFrame>
						</div>
					</div>

					<DialogPrimitive.Close
						className="absolute right-3 top-3 rounded-md p-1.5 text-muted-foreground hover:bg-foreground/[0.06] hover:text-foreground transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
						aria-label="Close settings"
					>
						<Icon icon={faClose} className="size-4" />
					</DialogPrimitive.Close>
				</DialogPrimitive.Content>
			</DialogPrimitive.Portal>
		</DialogPrimitive.Root>
	);
}

function NavItem({
	icon,
	label,
	active,
	onClick,
	trailing,
}: {
	icon: IconProp;
	label: string;
	active: boolean;
	onClick: () => void;
	trailing?: ReactNode;
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			className={cn(
				"w-full flex items-center gap-2 rounded-md px-2.5 py-1.5 text-sm text-left transition-colors",
				"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
				active
					? "bg-foreground/[0.06] text-foreground"
					: "text-muted-foreground hover:bg-foreground/[0.04] hover:text-foreground",
			)}
		>
			<Icon icon={icon} className="size-3 shrink-0" />
			<span className="truncate">{label}</span>
			{trailing}
		</button>
	);
}

// Renders the usage gauge next to the Billing nav item. Guarded with
// `shouldThrow: false` plus a `loaderData` check because the drawer can render
// while the project match tree is mid-transition; without it the inner billing
// hooks read undefined loader data and crash. Returns null off a project route
// so the project-scoped hooks never run.
function NavBillingGauge() {
	const match = useMatch({
		from: "/_context/orgs/$organization/projects/$project",
		shouldThrow: false,
	});
	if (!match || !match.loaderData) return null;
	return (
		<span className="ml-auto flex items-center shrink-0">
			<BillingUsageGauge />
		</span>
	);
}

function TabContent({ tab }: { tab: SettingsTab }) {
	switch (tab) {
		case "profile":
			return <ProfilePage />;
		case "billing":
			return <BillingPanel />;
		case "settings":
			return <SettingsTabBody />;
		case "advanced":
			return <AdvancedTabBody />;
		case "organization":
			return <OrganizationPanel />;
		case "whats-new":
			return <WhatsNewPanel />;
	}
}

function TabFrame({
	title,
	description,
	children,
}: {
	title: ReactNode;
	description?: string;
	children: React.ReactNode;
}) {
	return (
		<div className="mx-auto w-full max-w-4xl px-6 py-5">
			<div className="mb-5">
				<h2 className="text-base font-semibold text-foreground">
					{title}
				</h2>
				{description ? (
					<p className="mt-0.5 text-xs text-muted-foreground">
						{description}
					</p>
				) : null}
			</div>
			{children}
		</div>
	);
}

function NamespaceSettingsTitle({ fallback }: { fallback: string }) {
	// Guard with `shouldThrow: false` because the drawer can render this title
	// while the active match tree is mid-transition. The `loaderData` check
	// also handles the in-between state where the match has entered the tree
	// but its loader hasn't resolved — without it, the inner hook reads
	// undefined loader data and crashes when it touches `dataProvider`.
	const match = useMatch({
		from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
		shouldThrow: false,
	});
	if (!match || !match.loaderData) return <>{fallback}</>;
	return <NamespaceSettingsTitleInner fallback={fallback} />;
}

function NamespaceSettingsTitleInner({ fallback }: { fallback: string }) {
	const dataProvider = useCloudNamespaceDataProvider();
	if (!dataProvider) return <>{fallback}</>;
	const { data } = useQuery(dataProvider.currentNamespaceQueryOptions());
	const displayName = data?.displayName;
	return <>{displayName ? `${displayName} settings` : fallback}</>;
}

function ProjectBillingTitle({ fallback }: { fallback: string }) {
	const match = useMatch({
		from: "/_context/orgs/$organization/projects/$project",
		shouldThrow: false,
	});
	if (!match || !match.loaderData) return <>{fallback}</>;
	return <ProjectBillingTitleInner fallback={fallback} />;
}

function ProjectBillingTitleInner({ fallback }: { fallback: string }) {
	const dataProvider = useCloudProjectDataProvider();
	if (!dataProvider) return <>{fallback}</>;
	const { data } = useQuery(dataProvider.currentProjectQueryOptions());
	const displayName = data?.displayName;
	return <>{displayName ? `${displayName} Billing` : fallback}</>;
}

function SettingsTabBody() {
	// `useMatch` with `shouldThrow: false` returns `undefined` when the
	// namespace route is not in the active match tree. This is stricter than
	// `useMatchRoute` (which can flicker during transitions) and lets us bail
	// before any namespace-only hook gets called.
	const namespaceMatch = useMatch({
		from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
		shouldThrow: false,
	});

	if (!namespaceMatch) {
		return (
			<ResourcePicker
				title="Pick a namespace"
				description="Settings are scoped to a namespace. Choose one to manage providers, runners, and tokens."
				settings="settings"
				target="namespace"
			/>
		);
	}
	// During navigation from the resource picker the match enters the tree
	// before its loader resolves. Namespace-only hooks crash on undefined
	// loader data, so wait for it to land.
	if (!namespaceMatch.loaderData) {
		return <NamespaceSettingsSkeleton />;
	}
	return <NamespaceSettingsContent />;
}

function NamespaceSettingsSkeleton() {
	return (
		<div className="space-y-4">
			<div className="h-24 rounded-lg border border-foreground/10 bg-card/50" />
			<div className="h-40 rounded-lg border border-foreground/10 bg-card/50" />
			<div className="h-40 rounded-lg border border-foreground/10 bg-card/50" />
		</div>
	);
}

function AdvancedTabBody() {
	const namespaceMatch = useMatch({
		from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
		shouldThrow: false,
	});

	if (!namespaceMatch) {
		return (
			<ResourcePicker
				title="Pick a namespace"
				description="Advanced settings are scoped to a namespace. Choose one to manage tokens and datacenter status."
				settings="advanced"
				target="namespace"
			/>
		);
	}
	if (!namespaceMatch.loaderData) {
		return <NamespaceSettingsSkeleton />;
	}
	return <NamespaceAdvancedContent />;
}

export function settingsParamToTab(
	param: string | undefined,
): SettingsTab | null {
	switch (param) {
		case "profile":
		case "settings":
		case "advanced":
		case "billing":
		case "organization":
		case "whats-new":
			return param;
		// Legacy: members lived in its own tab before being merged into
		// Organization. Keep the deep link working.
		case "members":
			return "organization";
		default:
			return null;
	}
}

export function SettingsDrawerHost() {
	const navigate = useNavigate();
	const search = useSearch({ strict: false }) as Record<string, unknown>;
	const param =
		typeof search.settings === "string" ? search.settings : undefined;
	const tab = settingsParamToTab(param);

	return (
		<SettingsDrawer
			open={tab !== null}
			tab={tab ?? "profile"}
			onOpenChange={(open) => {
				if (!open) {
					navigate({
						to: ".",
						search: (old) => ({
							...(old as Record<string, unknown>),
							settings: undefined,
						}),
					});
				}
			}}
		/>
	);
}
