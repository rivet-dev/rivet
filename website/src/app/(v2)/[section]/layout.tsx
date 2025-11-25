import { Header } from "@/components/v2/Header";
import { findActiveTab, findPageForHref, Sitemap } from "@/lib/sitemap";
import { sitemap } from "@/sitemap/mod";
import { Button } from "@rivet-gg/components";
import Link from "next/link";
import type { CSSProperties, ReactNode } from "react";
import { buildFullPath, buildPathComponents } from "./util";
import { NavigationStateProvider } from "@/providers/NavigationStateProvider";
import { Tree } from "@/components/DocsNavigation";

function Subnav({ path }: { path: string[] }) {
	const fullPath = buildFullPath(path);
	return (
		<div className="hidden h-14 items-center empty:hidden md:flex gap-4 pt-2">
			{sitemap.map((tab, i) => {
				const isActive = findPageForHref(fullPath, tab);
				return (
					<Button
						// biome-ignore lint/suspicious/noArrayIndexKey: only used for static content
						key={i}
						variant="ghost"
						asChild
						className="text-muted-foreground aria-current-page:text-foreground px-0 text-sm hover:bg-transparent flex items-center border-b-2 border-transparent aria-current-page:border-primary rounded-none h-full"
					>
						<Link
							href={tab.href}
							target={tab.target}
							aria-current={isActive ? "page" : undefined}
						>
							{tab.title}
						</Link>
					</Button>
				);
			})}
		</div>
	);
}

export default function Layout({
	params: { section, page },
	children,
}: {
	params: { section: string; page?: string[] };
	children: ReactNode;
}) {
	const path = buildPathComponents(section, page);
	const fullPath = buildFullPath(path);
	const foundTab = findActiveTab(fullPath, sitemap as Sitemap);

	return (
		<NavigationStateProvider>
			<Header
				active="docs"
				subnav={<Subnav path={path} />}
				variant="full-width"
				mobileSidebar={
					foundTab?.tab.sidebar ? (
						<Tree
							className="mt-2 mb-4"
							pages={foundTab.tab.sidebar}
						/>
					) : null
				}
			/>
			<div className="w-full relative z-10 font-sans">
				<div
					className="mx-auto w-full min-h-content flex flex-col md:grid md:grid-cols-docs-no-sidebar lg:grid-cols-docs"
					style={{ "--header-height": "6.5rem" } as CSSProperties}
				>
					{children}
				</div>
			</div>
		</NavigationStateProvider>
	);
}

