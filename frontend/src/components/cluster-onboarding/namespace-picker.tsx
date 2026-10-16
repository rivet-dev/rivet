import {
	faCheck,
	faChevronDown,
	faChevronRight,
	faCirclePlus,
	faMagnifyingGlass,
	Icon,
} from "@rivet-gg/icons";
import { type ReactNode, useState } from "react";
import { cn } from "@/components/lib/utils";
import { Button } from "@/components/ui/button";
import {
	Popover,
	PopoverContent,
	PopoverTrigger,
} from "@/components/ui/popover";

/** Mock namespaces on the cluster. */
export const NAMESPACES = [
	{ id: "production", label: "production", actorCount: 4, color: "#22c55e" },
	{ id: "staging", label: "staging", actorCount: 4, color: "#eab308" },
	{ id: "dev-nathan", label: "dev-nathan", actorCount: 2, color: "#a855f7" },
] as const;

export type NamespaceId = (typeof NAMESPACES)[number]["id"];

/**
 * Four explorations of where the namespace switcher sits in the sidebar,
 * between the cluster-wide pages and the actor list:
 *
 * - `select`: eyebrow label + full-width bordered select, like a form field.
 * - `heading`: the "Actors" eyebrow itself becomes the dropdown (PlanetScale branches).
 * - `card`: a card with env color dot, name, and actor count, like a workspace switcher.
 * - `tree`: a collapsible group row; actors nest under it with a guide line.
 */
export type NamespaceVariant = "select" | "heading" | "card" | "tree";

export const NAMESPACE_VARIANTS: {
	id: NamespaceVariant;
	label: string;
	blurb: string;
}[] = [
	{
		id: "select",
		label: "A · Select field",
		blurb: "Eyebrow label + bordered select. Most explicit; reads as a form control.",
	},
	{
		id: "heading",
		label: "B · Heading dropdown",
		blurb: "The section label is the switcher. Lightest; PlanetScale branch style.",
	},
	{
		id: "card",
		label: "C · Card switcher",
		blurb: "Env color + actor count. Feels like a workspace/org switcher.",
	},
	{
		id: "tree",
		label: "D · Collapsible group",
		blurb: "Namespace is a tree node; actors nest under it. Scales to many namespaces.",
	},
];

interface NamespacePickerProps {
	variant: NamespaceVariant;
	value: NamespaceId;
	onChange: (id: NamespaceId) => void;
	/** The actor nav items, rendered below (or inside, for `tree`) the picker. */
	children: ReactNode;
}

export function NamespacePicker({
	variant,
	value,
	onChange,
	children,
}: NamespacePickerProps) {
	const current = NAMESPACES.find((n) => n.id === value) ?? NAMESPACES[0];

	if (variant === "select") {
		return (
			<div className="flex flex-col gap-0.5">
				<p className="px-1 pb-1 text-xs font-medium text-muted-foreground">
					Namespace
				</p>
				<NamespaceMenu value={value} onChange={onChange}>
					<button
						type="button"
						className="mb-2 flex w-full items-center gap-2 rounded-lg border border-foreground/10 bg-foreground/[0.02] px-2 py-1.5 text-sm font-medium hover:border-foreground/20 hover:bg-foreground/[0.06] data-[state=open]:bg-foreground/[0.06]"
					>
						<span className="flex-1 truncate text-left">
							{current.label}
						</span>
						<UnfoldIcon />
					</button>
				</NamespaceMenu>
				{children}
			</div>
		);
	}

	if (variant === "heading") {
		return (
			<div className="flex flex-col gap-0.5">
				<NamespaceMenu value={value} onChange={onChange}>
					<button
						type="button"
						className="mb-1 flex w-fit items-center gap-1.5 rounded-md px-1 py-0.5 text-xs font-medium text-muted-foreground hover:bg-foreground/[0.04] hover:text-foreground data-[state=open]:text-foreground"
					>
						Actors
						<span className="text-muted-foreground/50">/</span>
						<span className="text-foreground">{current.label}</span>
						<Icon
							icon={faChevronDown}
							className="size-2.5 opacity-60"
						/>
					</button>
				</NamespaceMenu>
				{children}
			</div>
		);
	}

	if (variant === "card") {
		return (
			<div className="flex flex-col gap-0.5">
				<NamespaceMenu value={value} onChange={onChange} showMeta>
					<button
						type="button"
						className="mb-2 flex w-full items-center gap-2.5 rounded-lg border border-foreground/10 bg-card px-2.5 py-2 text-left hover:border-foreground/20 hover:bg-foreground/[0.04] data-[state=open]:bg-foreground/[0.04]"
					>
						<span
							className="size-2 shrink-0 rounded-full"
							style={{ backgroundColor: current.color }}
						/>
						<span className="min-w-0 flex-1">
							<span className="block truncate text-sm font-medium leading-tight">
								{current.label}
							</span>
							<span className="block text-xs text-muted-foreground">
								{current.actorCount} actors · namespace
							</span>
						</span>
						<UnfoldIcon />
					</button>
				</NamespaceMenu>
				{children}
			</div>
		);
	}

	// tree
	return (
		<TreeGroup value={value} onChange={onChange}>
			{children}
		</TreeGroup>
	);
}

function TreeGroup({
	value,
	onChange,
	children,
}: {
	value: NamespaceId;
	onChange: (id: NamespaceId) => void;
	children: ReactNode;
}) {
	const [expanded, setExpanded] = useState(true);
	const current = NAMESPACES.find((n) => n.id === value) ?? NAMESPACES[0];
	return (
		<div className="flex flex-col gap-0.5">
			<div className="flex items-center">
				<Button
					variant="ghost"
					className="h-auto flex-1 justify-start gap-1.5 px-1 py-1 font-medium text-foreground hover:bg-foreground/[0.04]"
					onClick={() => setExpanded((e) => !e)}
					aria-expanded={expanded}
				>
					<Icon
						icon={expanded ? faChevronDown : faChevronRight}
						className="size-2.5 text-muted-foreground"
					/>
					<span className="truncate">{current.label}</span>
					<span className="text-xs font-normal text-muted-foreground">
						{current.actorCount}
					</span>
				</Button>
				<NamespaceMenu value={value} onChange={onChange}>
					<Button
						variant="ghost"
						size="icon-sm"
						aria-label="Switch namespace"
						className="text-muted-foreground hover:text-foreground data-[state=open]:bg-foreground/[0.06]"
					>
						<UnfoldIcon />
					</Button>
				</NamespaceMenu>
			</div>
			{expanded ? (
				<div className="relative ml-2.5 flex flex-col gap-0.5 border-l border-border pl-1.5">
					{children}
				</div>
			) : null}
		</div>
	);
}

/** Shared popover list: search, namespaces with check mark, create row. */
function NamespaceMenu({
	value,
	onChange,
	showMeta,
	children,
}: {
	value: NamespaceId;
	onChange: (id: NamespaceId) => void;
	showMeta?: boolean;
	children: ReactNode;
}) {
	const [open, setOpen] = useState(false);
	const [query, setQuery] = useState("");
	const items = NAMESPACES.filter((n) =>
		n.label.toLowerCase().includes(query.toLowerCase()),
	);
	return (
		<Popover
			open={open}
			onOpenChange={(o) => {
				setOpen(o);
				if (!o) setQuery("");
			}}
		>
			<PopoverTrigger asChild>{children}</PopoverTrigger>
			<PopoverContent className="w-60 p-1" align="start">
				<label className="flex items-center gap-2 border-b px-2 py-1.5 text-sm">
					<Icon
						icon={faMagnifyingGlass}
						className="size-3 text-muted-foreground"
					/>
					<input
						value={query}
						onChange={(e) => setQuery(e.target.value)}
						placeholder="Find namespace…"
						className="w-full bg-transparent outline-none placeholder:text-muted-foreground"
					/>
				</label>
				<p className="px-2 pb-1 pt-2 text-xs font-medium text-muted-foreground">
					Namespaces
				</p>
				{items.map((n) => (
					<button
						key={n.id}
						type="button"
						onClick={() => {
							onChange(n.id);
							setOpen(false);
						}}
						className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-accent"
					>
						{showMeta ? (
							<span
								className="size-2 shrink-0 rounded-full"
								style={{ backgroundColor: n.color }}
							/>
						) : null}
						<span className="flex-1 truncate text-left">
							{n.label}
						</span>
						{showMeta ? (
							<span className="text-xs text-muted-foreground">
								{n.actorCount}
							</span>
						) : null}
						{n.id === value ? (
							<Icon
								icon={faCheck}
								className="size-3 text-primary"
							/>
						) : null}
					</button>
				))}
				{items.length === 0 ? (
					<p className="px-2 py-1.5 text-sm text-muted-foreground">
						No namespaces match.
					</p>
				) : null}
				<div className="my-1 border-t" />
				<button
					type="button"
					className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-muted-foreground hover:bg-accent hover:text-foreground"
				>
					<Icon icon={faCirclePlus} className="size-3.5" />
					Create namespace
				</button>
			</PopoverContent>
		</Popover>
	);
}

function UnfoldIcon({ className }: { className?: string }) {
	return (
		<svg
			viewBox="0 0 24 24"
			width="14"
			height="14"
			stroke="currentColor"
			strokeWidth="2"
			strokeLinecap="round"
			strokeLinejoin="round"
			fill="none"
			className={cn("size-3 shrink-0 opacity-60", className)}
			role="img"
			aria-label="Switch"
		>
			<path d="m7 15 5 5 5-5" />
			<path d="m7 9 5-5 5 5" />
		</svg>
	);
}
