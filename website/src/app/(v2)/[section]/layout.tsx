import { Header } from "@/components/v2/Header";
import { findActiveTab, Sitemap } from "@/lib/sitemap";
import { sitemap } from "@/sitemap/mod";
import type { CSSProperties, ReactNode } from "react";
import { buildFullPath, buildPathComponents } from "./util";
import { NavigationStateProvider } from "@/providers/NavigationStateProvider";
import { Tree } from "@/components/DocsNavigation";
import { Subnav } from "./Subnav";

export default async function Layout({
	params,
	children,
}: {
	params: Promise<{ section: string; page?: string[] }>;
	children: ReactNode;
}) {
	const { section, page } = await params;
	const path = buildPathComponents(section, page);
	const fullPath = buildFullPath(path);
	const foundTab = findActiveTab(fullPath, sitemap as Sitemap);

	return (
		<NavigationStateProvider>
			<Header
				active="docs"
				subnav={<Subnav />}
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
			<div className="w-full relative z-10 font-sans selection:bg-[#FF4500]/30 selection:text-orange-200">
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

