import { Header } from "@/components/v2/Header";
import { findActiveTab, type Sitemap } from "@/lib/sitemap";
import { sitemap } from "@/sitemap/mod";
import type { CSSProperties } from "react";
import { buildFullPath, buildPathComponents } from "./util";
import { NavigationStateProvider } from "@/providers/NavigationStateProvider";
import { Tree } from "@/components/DocsNavigation";
import { Subnav } from "./components/subnav";

export default function Layout({ params: { section, page }, children }) {
	const path = buildPathComponents(section, page);
	const fullPath = buildFullPath(path);
	const foundTab = findActiveTab(fullPath, sitemap as Sitemap);

	return (
		<NavigationStateProvider>
			<Header
				active="docs"
				subnav={<Subnav path={fullPath} />}
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
			<div className="w-full relative z-10">
				<div
					className="md:grid-cols-docs-no-sidebar lg:grid-cols-docs mx-auto flex w-full flex-col justify-center md:grid min-h-content"
					style={{ "--header-height": "6.5rem" } as CSSProperties}
				>
					{children}
				</div>
			</div>
		</NavigationStateProvider>
	);
}
