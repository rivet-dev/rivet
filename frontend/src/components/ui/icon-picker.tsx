"use client";

import { faMagnifyingGlass, faQuestion, Icon, type IconProp } from "@rivet-gg/icons";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
	type ComponentPropsWithoutRef,
	forwardRef,
	type LazyExoticComponent,
	lazy,
	type ReactNode,
	Suspense,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import { cn } from "../lib/utils";
import { Button } from "./button";
import { Input } from "./input";
import { Popover, PopoverContent, PopoverTrigger } from "./popover";

const iconModules = import.meta.glob<Record<string, IconProp>>(
	"../../../packages/icons/dist/icons/*.js",
);

function capitalize(s: string): string {
	return s.charAt(0).toUpperCase() + s.slice(1);
}

function toExportName(iconName: string): string {
	return `fa${iconName.split("-").map(capitalize).join("")}`;
}

const lazyIconCache = new Map<
	string,
	LazyExoticComponent<(props: { className?: string }) => ReactNode>
>();

function getLazyIcon(iconName: string) {
	const exportName = toExportName(iconName);
	const cached = lazyIconCache.get(exportName);
	if (cached) return cached;

	const loader = iconModules[`../../../packages/icons/dist/icons/${exportName}.js`];
	const component = lazy(() =>
		(loader ? loader() : Promise.reject())
			.then((mod) => ({
				default: ({ className }: { className?: string }) => (
					<Icon
						icon={mod[exportName] ?? faQuestion}
						className={className}
					/>
				),
			}))
			.catch(() => ({
				default: ({ className }: { className?: string }) => (
					<Icon icon={faQuestion} className={className} />
				),
			})),
	);
	lazyIconCache.set(exportName, component);
	return component;
}

export function IconRenderer({
	name,
	className,
	fallback,
}: {
	name: string | null | undefined;
	className?: string;
	fallback?: ReactNode;
}) {
	if (!name) return <>{fallback ?? null}</>;
	const LazyIcon = getLazyIcon(name);
	return (
		<Suspense fallback={fallback ?? <span className={className} />}>
			<LazyIcon className={className} />
		</Suspense>
	);
}

interface IconPickerProps {
	value?: string | null;
	onChange: (iconName: string | null) => void;
	trigger?: ReactNode;
	columns?: number;
	cellSize?: number;
	className?: string;
}

export function IconPicker({
	value,
	onChange,
	trigger,
	columns = 8,
	cellSize = 36,
	className,
}: IconPickerProps) {
	const [open, setOpen] = useState(false);

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				{trigger ?? <DefaultTrigger iconName={value ?? null} />}
			</PopoverTrigger>
			<PopoverContent className={cn("w-80 p-0", className)} align="start">
				{open ? (
					<IconPickerBody
						value={value ?? null}
						onChange={(name) => {
							onChange(name);
							setOpen(false);
						}}
						columns={columns}
						cellSize={cellSize}
					/>
				) : null}
			</PopoverContent>
		</Popover>
	);
}

const DefaultTrigger = forwardRef<
	HTMLButtonElement,
	ComponentPropsWithoutRef<"button"> & { iconName: string | null }
>(({ iconName, ...rest }, ref) => (
	<Button
		ref={ref}
		variant="outline"
		size="icon"
		type="button"
		aria-label={iconName ? `Icon: ${iconName}` : "Pick an icon"}
		{...rest}
	>
		{iconName ? (
			<IconRenderer name={iconName} className="size-4" />
		) : (
			<Icon icon={faMagnifyingGlass} className="size-4 opacity-50" />
		)}
	</Button>
));
DefaultTrigger.displayName = "IconPickerDefaultTrigger";

interface IconEntry {
	key: string;
	iconName: string;
	def: IconProp;
	searchTerms: string[];
}

let cachedEntriesPromise: Promise<IconEntry[]> | null = null;

function loadAllIcons(): Promise<IconEntry[]> {
	if (cachedEntriesPromise) return cachedEntriesPromise;
	cachedEntriesPromise = import("@rivet-gg/icons").then((mod) => {
		const seen = new Set<string>();
		const out: IconEntry[] = [];
		for (const value of Object.values(mod as Record<string, unknown>)) {
			if (!isIconDef(value)) continue;
			const key = `${value.prefix}:${value.iconName}`;
			if (seen.has(key)) continue;
			seen.add(key);
			out.push({
				key,
				iconName: value.iconName,
				def: value as unknown as IconProp,
				searchTerms: value.iconName.split("-"),
			});
		}
		out.sort((a, b) => a.iconName.localeCompare(b.iconName));
		return out;
	});
	return cachedEntriesPromise;
}

function isIconDef(value: unknown): value is {
	prefix: string;
	iconName: string;
	icon: unknown[];
} {
	if (!value || typeof value !== "object") return false;
	const v = value as Record<string, unknown>;
	return (
		typeof v.prefix === "string" &&
		typeof v.iconName === "string" &&
		Array.isArray(v.icon)
	);
}

interface IconPickerBodyProps {
	value: string | null;
	onChange: (iconName: string | null) => void;
	columns: number;
	cellSize: number;
}

function IconPickerBody({ value, onChange, columns, cellSize }: IconPickerBodyProps) {
	const [query, setQuery] = useState("");
	const [entries, setEntries] = useState<IconEntry[] | null>(null);
	const inputRef = useRef<HTMLInputElement>(null);
	const scrollRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		inputRef.current?.focus();
		let cancelled = false;
		loadAllIcons().then((all) => {
			if (!cancelled) setEntries(all);
		});
		return () => {
			cancelled = true;
		};
	}, []);

	const filtered = useMemo(() => {
		if (!entries) return [];
		const q = query.trim().toLowerCase();
		if (!q) return entries;
		return entries.filter(
			(e) =>
				e.iconName.includes(q) ||
				e.searchTerms.some((t) => t.includes(q)),
		);
	}, [entries, query]);

	const rowCount = Math.ceil(filtered.length / columns);
	const rowVirtualizer = useVirtualizer({
		count: rowCount,
		getScrollElement: () => scrollRef.current,
		estimateSize: () => cellSize,
		overscan: 4,
	});

	return (
		<div className="flex flex-col">
			<div className="p-2 border-b flex items-center gap-2">
				<Input
					ref={inputRef}
					value={query}
					onChange={(e) => setQuery(e.target.value)}
					placeholder="Search icons"
					className="h-8"
				/>
				{value ? (
					<Button
						type="button"
						variant="ghost"
						size="sm"
						onClick={() => onChange(null)}
					>
						Clear
					</Button>
				) : null}
			</div>
			<div className="px-3 py-1 text-xs text-muted-foreground">
				{entries === null
					? "Loading icons…"
					: `${filtered.length} icon${filtered.length === 1 ? "" : "s"}`}
			</div>
			<div
				ref={scrollRef}
				className="h-72 overflow-y-auto overflow-x-hidden px-2 pb-2"
			>
				{entries === null ? null : filtered.length === 0 ? (
					<div className="h-full flex items-center justify-center text-sm text-muted-foreground">
						No icons match "{query}"
					</div>
				) : (
					<div
						className="relative w-full"
						style={{ height: `${rowVirtualizer.getTotalSize()}px` }}
					>
						{rowVirtualizer.getVirtualItems().map((row) => {
							const start = row.index * columns;
							const rowItems = filtered.slice(start, start + columns);
							return (
								<div
									key={row.key}
									className="absolute inset-x-0 grid"
									style={{
										gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
										transform: `translateY(${row.start}px)`,
										height: `${cellSize}px`,
									}}
								>
									{rowItems.map((entry) => (
										<IconCell
											key={entry.key}
											entry={entry}
											selected={entry.iconName === value}
											onSelect={() => onChange(entry.iconName)}
										/>
									))}
								</div>
							);
						})}
					</div>
				)}
			</div>
		</div>
	);
}

interface IconCellProps extends ComponentPropsWithoutRef<"button"> {
	entry: IconEntry;
	selected: boolean;
	onSelect: () => void;
}

function IconCell({ entry, selected, onSelect, className, ...rest }: IconCellProps) {
	return (
		<button
			type="button"
			onClick={onSelect}
			title={entry.iconName}
			aria-label={entry.iconName}
			aria-pressed={selected}
			className={cn(
				"flex items-center justify-center rounded-md hover:bg-accent transition-colors",
				selected && "bg-accent ring-1 ring-primary",
				className,
			)}
			{...rest}
		>
			<Icon icon={entry.def} className="size-4" />
		</button>
	);
}
