import { ModulePageLink } from "@/components/ModulePageLink";
import { Header } from "@/components/v2/Header";
import { findPageForHref } from "@/lib/sitemap";
import { sitemap } from "@/sitemap/mod";
import type { CSSProperties } from "react";
import { buildFullPath, buildPathComponents } from "./util";

function Subnav({ path }: { path: string[] }) {
	const fullPath = buildFullPath(path);
	return (
		<div className="-mx-8 -mb-[9px] hidden min-h-10 items-center px-8 empty:hidden md:flex">
			{sitemap.map((tab, i) => (
				<ModulePageLink
					// biome-ignore lint/suspicious/noArrayIndexKey: only used for static content
					key={i}
					href={tab.href}
					target={tab.target}
					isActive={findPageForHref(fullPath, tab)}
				>
					{tab.title}
				</ModulePageLink>
			))}
		</div>
	);
}

export default function Layout({ params: { section, page }, children }) {
	const path = buildPathComponents(section, page);
	return (
		<>
			<Header active="docs" subnav={<Subnav path={path} />} />
			<div className="w-full">
				<div
					className="md:grid-cols-docs mx-auto flex w-full flex-col justify-center md:grid md:px-6"
					style={{ "--header-height": "6.5rem" } as CSSProperties}
				>
					{children}
				</div>
			</div>
		</>
	);
}
