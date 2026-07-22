import { ActiveLink, SsrPathnameContext } from "@/components/ActiveLink";
import { CollapsibleSidebarItem } from "@/components/CollapsibleSidebarItem";
import { NavigationStateProvider } from "@/providers/NavigationStateProvider";
import routes from "@/generated/routes.json";
import type { SidebarItem } from "@/lib/sitemap";
import { cn } from "@rivet-gg/components";
import { Icon, faArrowUpRight } from "@rivet-gg/icons";
import clsx from "clsx";
import { type PropsWithChildren, type ReactNode, useEffect, useRef } from "react";

interface TreeItemProps {
	index: number;
	item: SidebarItem;
	level?: number;
	parentPath?: string;
}

function TreeItem({ index, item, level = 0, parentPath = "" }: TreeItemProps) {
	if (
		"collapsible" in item &&
		"title" in item &&
		"pages" in item &&
		item.collapsible
	) {
		const itemPath = parentPath
			? `${parentPath}.${item.title}`
			: item.title;
		return (
			<CollapsibleSidebarItem
				item={item}
				level={level}
				parentPath={parentPath}
			>
				<Tree
					pages={item.pages}
					level={level + 1}
					parentPath={itemPath}
				/>
			</CollapsibleSidebarItem>
		);
	}

	if ("title" in item && "pages" in item) {
		const itemPath = parentPath
			? `${parentPath}.${item.title}`
			: item.title;
		return (
			<div>
				<p
					className={clsx(
						"mb-3 text-[15px] font-semibold text-ink flex items-center",
						index > 0 ? "mt-6" : undefined,
					)}
				>
					{item.icon ? (
						<Icon icon={item.icon} className="mr-2 size-3.5" />
					) : null}
					<span className="truncate"> {item.title}</span>
					{"badge" in item && item.badge ? (
						<span className="ml-2 px-2 py-0.5 text-xs font-medium bg-ink/[0.06] text-ink-soft rounded-full whitespace-nowrap">
							{item.badge}
						</span>
					) : null}
				</p>
				<Tree
					pages={item.pages}
					level={level + 1}
					parentPath={itemPath}
				/>
			</div>
		);
	}

	return (
		<NavLink href={item.href} external={item.external} level={level}>
			<span className="flex items-center truncate gap-2">
				{item.icon && "prefix" in item.icon ? (
					<Icon icon={item.icon} className="size-3.5 flex-shrink-0" />
				) : item.icon ? (
					<img
						{...(item.icon as Record<string, string>)}
						className="h-3.5 w-auto max-w-8 flex-shrink-0"
						alt=""
					/>
				) : null}
				<span className="truncate">
					{item.title ?? routes.pages[getAliasedHref(item.href)]?.title}
				</span>
			</span>
			{"badge" in item && item.badge ? (
				<span className="ml-2 px-[6px] py-0 text-[10px] font-medium bg-ink/[0.06] border border-ink/10 text-ink-soft rounded-sm whitespace-nowrap">
					{item.badge}
				</span>
			) : null}
			{item.external ? (
				<Icon icon={faArrowUpRight} className="ml-2 size-3" />
			) : null}
		</NavLink>
	);
}

interface TreeProps {
	pages: SidebarItem[];
	className?: string;
	level?: number;
	parentPath?: string;
}

export function Tree({
	pages,
	className,
	level = 0,
	parentPath = "",
}: TreeProps) {
	return (
		<ul className={cn(className)}>
			{pages.map((item, index) => (
				<li
					// biome-ignore lint/suspicious/noArrayIndexKey: FIXME: used only for static content
					key={index}
				>
					<TreeItem
						index={index}
						item={item}
						level={level}
						parentPath={parentPath}
					/>
				</li>
			))}
		</ul>
	);
}

export function NavLink({
	href,
	external,
	children,
	className,
	level = 0,
}: PropsWithChildren<{
	href: string;
	external?: boolean;
	children: ReactNode;
	className?: string;
	level?: number;
}>) {
	const getPaddingClass = (level: number) => {
		switch (level) {
			case 0:
				return "pl-2 pr-3";
			case 1:
				return "pl-5 pr-3";
			case 2:
				return "pl-8 pr-3";
			default:
				return "pl-11 pr-3";
		}
	};

	return (
		<ActiveLink
			strict
			href={href}
			target={external && "_blank"}
			className={cn(
				"group flex w-full items-center border-l-2 border-l-ink/15 py-2 text-sm text-ink-soft transition-colors hover:text-ink hover:border-l-ink/30 aria-current-page:text-ink aria-current-page:border-l-pine",
				getPaddingClass(level),
				className,
			)}
		>
			{children}
		</ActiveLink>
	);
}

export function DocsNavigation({
	sidebar,
	tabKey,
	currentPath = "",
}: { sidebar: SidebarItem[]; tabKey?: string; currentPath?: string }) {
	const scrollRef = useRef<HTMLDivElement>(null);
	const prevTabKey = useRef(tabKey);

	// The sidebar island is persisted across view transitions to keep its scroll
	// position, so it does not remount when navigating within a tab. Reset the
	// scroll only when moving to a different tab, whose sidebar is unrelated.
	useEffect(() => {
		if (prevTabKey.current !== tabKey) {
			prevTabKey.current = tabKey;
			if (scrollRef.current) {
				scrollRef.current.scrollTop = 0;
			}
		}
	}, [tabKey]);

	return (
		<SsrPathnameContext.Provider value={currentPath}>
			<NavigationStateProvider>
				<div
					ref={scrollRef}
					className="sticky top-header max-h-content text-ink pl-8 pr-6 py-6 overflow-y-auto [mask-image:linear-gradient(to_bottom,transparent_0,#000_2rem,#000_calc(100%-1.5rem),transparent_100%)] [-webkit-mask-image:linear-gradient(to_bottom,transparent_0,#000_2rem,#000_calc(100%-1.5rem),transparent_100%)]"
				>
					<Tree pages={sidebar} />
				</div>
			</NavigationStateProvider>
		</SsrPathnameContext.Provider>
	);
}

export function getAliasedHref(href: string) {
	const [_, __, ...slug] = href.split("/");
	return `/docs/${slug.join("/")}`;
}
