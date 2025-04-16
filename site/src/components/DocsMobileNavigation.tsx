"use client";

import { ActiveLink } from "@/components/ActiveLink";
import { Tree } from "@/components/DocsNavigation";
import { sitemap } from "@/sitemap/mod";
import { Header as RivetHeader } from "@rivet-gg/components/header";
import { usePathname } from "next/navigation";

function CoreNavigation() {}

const links = [
	{ href: "/docs", label: "Docs" },
	//{ href: "/product", label: "Product" },
	{ href: "/changelog", label: "Changelog" },
	{ href: "/pricing", label: "Pricing" },
];

export function DocsMobileNavigation() {
	const pathname = usePathname() || "";

	const currentSidebar = sitemap.find((page) =>
		pathname.startsWith(page.href),
	);
	return (
		<>
			{links.map(({ href, external, label }) =>
				pathname.startsWith(href) && currentSidebar ? (
					<div className="flex flex-col gap-1" key={href}>
						<RivetHeader.NavItem
							asChild
							className="flex items-center gap-1.5"
						>
							<ActiveLink
								href={href}
								target={external && "_blank"}
							>
								{label}
							</ActiveLink>
						</RivetHeader.NavItem>
						<Tree pages={currentSidebar.sidebar} />
					</div>
				) : (
					<RivetHeader.NavItem
						key={href}
						asChild
						className="flex items-center gap-1.5"
					>
						<ActiveLink href={href} target={external && "_blank"}>
							{label}
						</ActiveLink>
					</RivetHeader.NavItem>
				),
			)}
		</>
	);
}
