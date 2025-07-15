import { ActiveLink } from "@/components/ActiveLink";
import { CollapsibleSidebarItem } from "@/components/CollapsibleSidebarItem";
import routes from "@/generated/routes.json";
import type { SidebarItem } from "@/lib/sitemap";
import { cn } from "@rivet-gg/components";
import { Icon, faArrowUpRight } from "@rivet-gg/icons";
import type { PropsWithChildren, ReactNode } from "react";

interface TreeItemProps {
	item: SidebarItem;
	level?: number;
}

function TreeItem({ item, level = 0 }: TreeItemProps) {
	if (
		"collapsible" in item &&
		"title" in item &&
		"pages" in item &&
		item.collapsible
	) {
		return (
			<CollapsibleSidebarItem item={item} level={level}>
				<Tree pages={item.pages} level={level + 1} />
			</CollapsibleSidebarItem>
		);
	}

	if ("title" in item && "pages" in item) {
		return (
			<div>
				<p className="mt-2 py-2 text-sm font-semibold">
					{item.icon ? (
						<Icon icon={item.icon} className="mr-2 size-3.5" />
					) : null}
					<span className="truncate"> {item.title}</span>
				</p>
				<Tree pages={item.pages} level={level + 1} />
			</div>
		);
	}

	return (
		<NavLink href={item.href} external={item.external} level={level}>
			{item.icon ? (
				<Icon icon={item.icon} className="mr-2 size-3.5" />
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
}

export function Tree({ pages, className, level = 0 }: TreeProps) {
	return (
		<ul className={cn(className)}>
			{pages.map((item, index) => (
				<li
					// biome-ignore lint/suspicious/noArrayIndexKey: FIXME: used only for static content
					key={index}
				>
					<TreeItem item={item} level={level} />
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
			case 0: return "pl-3 pr-3";
			case 1: return "pl-6 pr-3";
			case 2: return "pl-9 pr-3";
			default: return "pl-12 pr-3";
		}
	};
	
	return (
		<ActiveLink
			strict
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
		<div className="top-header sticky pr-4 text-white md:max-h-content md:overflow-y-auto md:pb-4 md:pt-8">
			<Tree pages={sidebar} />
		</div>
	);
}

export function getAliasedHref(href: string) {
	const [_, __, ...slug] = href.split("/");
	return `/docs/${slug.join("/")}`;
}
