"use client";

import { findPageForHref } from "@/lib/sitemap";
import { sitemap } from "@/sitemap/mod";
import { Button } from "@rivet-gg/components";
import Link from "next/link";
import { usePathname } from "next/navigation";

export function Subnav() {
	const pathname = usePathname() || "";
	// Remove trailing slash for consistency
	const normalizedPath = pathname.replace(/\/$/, "");

	return (
		<div className="hidden h-14 items-center empty:hidden md:flex gap-4 pt-2">
			{sitemap.map((tab, i) => {
				const isActive = findPageForHref(normalizedPath, tab);
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
