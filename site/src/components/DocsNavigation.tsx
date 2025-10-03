import { ActiveLink } from "@/components/ActiveLink";
import { CollapsibleSidebarItem } from "@/components/CollapsibleSidebarItem";
import { DynamicNavWrapper } from "@/components/DynamicNavWrapper";
import routes from "@/generated/routes.json";
import type { SidebarItem } from "@/lib/sitemap";
import { cn } from "@rivet-gg/components";
import { Icon, faArrowUpRight } from "@rivet-gg/icons";
import clsx from "clsx";
import type { PropsWithChildren, ReactNode } from "react";

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
						"mb-2 text-sm font-semibold",
						index > 0 ? "mt-4" : undefined,
					)}
				>
					{item.icon ? (
						<Icon icon={item.icon} className="mr-2 size-3.5" />
					) : null}
					<span className="truncate"> {item.title}</span>
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
			{item.icon && "prefix" in item.icon ? (
				<Icon icon={item.icon} className="mr-2 size-3.5" />
			) : item.icon ? (
				<img
					{...(item.icon as Record<string, string>)}
					className="mr-2 size-3.5"
					alt={item.title}
				/>
			) : null}
			<span className="truncate">
				{item.title ?? routes.pages[getAliasedHref(item.href)]?.title}
			</span>
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
				return "pl-3 pr-3";
			case 1:
				return "pl-6 pr-3";
			case 2:
				return "pl-9 pr-3";
			default:
				return "pl-12 pr-3";
		}
	};

	return (
		<ActiveLink
			strict="true"
			href={href}
			target={external && "_blank"}
			className={cn(
				"group flex w-full items-center border-l-2 border-l-border py-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground hover:border-l-muted-foreground/50 aria-current-page:text-foreground aria-current-page:border-l-orange-500",
				getPaddingClass(level),
				className,
			)}
		>
			{children}
		</ActiveLink>
	);
}

export function DocsNavigation({ sidebar }: { sidebar: SidebarItem[] }) {
	return (
		<DynamicNavWrapper className="sticky text-white pl-8 pr-6 py-6 overflow-y-auto">
			<Tree pages={sidebar} />
		</DynamicNavWrapper>
	);
}

export function getAliasedHref(href: string) {
	const [_, __, ...slug] = href.split("/");
	return `/docs/${slug.join("/")}`;
}
