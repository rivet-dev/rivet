import {
	faActors,
	faCreditCard,
	faDiagramProject,
	faGear,
	faHouse,
	faLogs,
	faPlus,
	faRobot,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/components/lib/utils";
import { ProductMark } from "@/components/products/product-picker";
import {
	type NamespaceId,
	NamespacePicker,
	type NamespaceVariant,
} from "./namespace-picker";

export type ClusterView =
	| "overview"
	| "logs"
	| "billing"
	| "settings"
	| ActorView;

export const CLUSTER_VIEWS: {
	id: ClusterView;
	label: string;
	icon: IconProp;
}[] = [
	{ id: "overview", label: "Overview", icon: faHouse },
	{ id: "logs", label: "Logs", icon: faLogs },
	{ id: "billing", label: "Billing", icon: faCreditCard },
	{ id: "settings", label: "Settings", icon: faGear },
];

/** Rivet products that run as actors on the cluster, each with its brand mark. */
export const PRODUCT_ACTORS = [
	{
		id: "actor:agentos",
		label: "agentOS",
		markFileName: "agentos-mark.svg",
		section: "products",
	},
	{
		id: "actor:dynamic-apps",
		label: "Dynamic Apps",
		markFileName: "dynamic-apps-mark.svg",
		section: "products",
	},
	{
		id: "actor:workflows",
		label: "Workflows",
		markFileName: "workflows-mark.svg",
		section: "products",
	},
	{
		id: "actor:durable-streams",
		label: "Durable Streams",
		markFileName: "durable-streams-mark.svg",
		section: "services",
	},
] as const;

/** Actors the user deployed themselves; in the real dashboard these come from the registry. */
export const USER_ACTORS = [
	{ id: "actor:myUser", label: "myUser", icon: faActors },
	{ id: "actor:myOrg", label: "myOrg", icon: faActors },
	{ id: "actor:myAgent", label: "myAgent", icon: faRobot },
	{ id: "actor:myWorkflow", label: "myWorkflow", icon: faDiagramProject },
] as const;

/** Quickstart page for writing and deploying your own actor. */
export const DEPLOY_ACTOR = {
	id: "actor:deploy",
	label: "Deploy an Actor",
	markFileName: "actors-mark.svg",
	section: "products",
} as const;

export type ActorView =
	| (typeof PRODUCT_ACTORS)[number]["id"]
	| (typeof USER_ACTORS)[number]["id"]
	| typeof DEPLOY_ACTOR.id;

export function isActorView(view: ClusterView): view is ActorView {
	return view.startsWith("actor:");
}

export function userActorIcon(view: ActorView): IconProp {
	return USER_ACTORS.find((a) => a.id === view)?.icon ?? faActors;
}

export function actorViewLabel(view: ActorView): string {
	return (
		[...PRODUCT_ACTORS, ...USER_ACTORS, DEPLOY_ACTOR].find(
			(a) => a.id === view,
		)?.label ?? view
	);
}

interface ClusterSidebarProps {
	view: ClusterView;
	onViewChange: (view: ClusterView) => void;
	/** Which namespace-switcher exploration to render. See `NamespacePicker`. */
	namespaceVariant?: NamespaceVariant;
	namespace?: NamespaceId;
	onNamespaceChange?: (id: NamespaceId) => void;
}

/**
 * Left sidebar, same chrome as the real dashboard `Sidebar` (ghost nav
 * buttons).
 */
export function ClusterSidebar({
	view,
	onViewChange,
	namespaceVariant = "heading",
	namespace = "production",
	onNamespaceChange = () => {},
}: ClusterSidebarProps) {
	// dev-nathan is a smaller namespace so switching visibly changes the list.
	const userActors =
		namespace === "dev-nathan" ? USER_ACTORS.slice(2) : USER_ACTORS;
	return (
		<aside className="flex w-56 shrink-0 flex-col gap-3 bg-background px-2 pt-1.5">
			<nav className="flex flex-col gap-0.5">
				{CLUSTER_VIEWS.map((item) => (
					<SidebarButton
						key={item.id}
						icon={<Icon className="size-5" icon={item.icon} />}
						active={view === item.id}
						onClick={() => onViewChange(item.id)}
					>
						{item.label}
					</SidebarButton>
				))}
			</nav>
			<div className="mx-1 border-t" />
			<nav aria-label="Actors" className="flex flex-col gap-0.5">
				<NamespacePicker
					variant={namespaceVariant}
					value={namespace}
					onChange={onNamespaceChange}
				>
					{namespaceVariant === "select" ||
					namespaceVariant === "card" ? (
						<p className="px-1 pb-1 text-xs font-medium text-muted-foreground">
							Actors
						</p>
					) : null}
					{PRODUCT_ACTORS.map((item) => (
						<SidebarButton
							key={item.id}
							icon={
								<ProductMark
									fileName={item.markFileName}
									section={item.section}
									size="sm"
								/>
							}
							active={view === item.id}
							onClick={() => onViewChange(item.id)}
						>
							{item.label}
						</SidebarButton>
					))}
					{userActors.map((item) => (
						<SidebarButton
							key={item.id}
							icon={
								<span className="flex size-4 items-center justify-center rounded bg-muted">
									<Icon className="size-3" icon={item.icon} />
								</span>
							}
							active={view === item.id}
							onClick={() => onViewChange(item.id)}
						>
							{item.label}
						</SidebarButton>
					))}
					<SidebarButton
						icon={<Icon className="size-4" icon={faPlus} />}
						active={view === DEPLOY_ACTOR.id}
						onClick={() => onViewChange(DEPLOY_ACTOR.id)}
						className="text-muted-foreground/60"
					>
						{DEPLOY_ACTOR.label}
					</SidebarButton>
				</NamespacePicker>
			</nav>
		</aside>
	);
}

/** Mirrors `HeaderLink` in `@/app/layout`: ghost button, active = foreground tint. */
function SidebarButton({
	icon,
	active,
	disabled,
	className,
	children,
	onClick,
}: {
	icon: ReactNode;
	active: boolean;
	disabled?: boolean;
	className?: string;
	children: ReactNode;
	onClick?: () => void;
}) {
	return (
		<Button
			variant="ghost"
			// `data-active:` styles key off the attribute TanStack `Link` sets.
			data-status={active ? "active" : undefined}
			aria-current={active ? "page" : undefined}
			disabled={disabled}
			onClick={onClick}
			className={cn(
				"h-auto justify-start px-1 py-1 font-medium text-muted-foreground hover:bg-foreground/[0.04] data-active:bg-foreground/[0.06] data-active:text-foreground",
				className,
			)}
			startIcon={
				<span className="flex size-5 shrink-0 items-center justify-center">
					{icon}
				</span>
			}
		>
			{children}
		</Button>
	);
}
