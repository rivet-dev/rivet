"use client";

import { usePathname } from "@/hooks/usePathname";
import { findPageForHref } from "@/lib/sitemap";
import { sitemap } from "@/sitemap/mod";
import { cn } from "@rivet-gg/components";

export function DocsTabs() {
	const pathname = usePathname() || "";
	// Remove trailing slash for consistency
	const normalizedPath = pathname.replace(/\/$/, "");

	return (
		<div className="hidden h-14 items-center empty:hidden md:flex gap-4 pt-2">
			{sitemap.map((tab) => {
				const isActive = findPageForHref(normalizedPath, tab);
				return (
					<a
						key={tab.href}
						href={tab.href}
						target={tab.target}
						aria-current={isActive ? "page" : undefined}
						className={cn(
							"text-muted-foreground px-0 text-sm hover:bg-transparent flex items-center border-b-2 border-transparent rounded-none h-full transition-colors",
							"aria-[current=page]:text-foreground aria-[current=page]:border-primary"
						)}
					>
						{tab.title}
					</a>
				);
			})}
		</div>
	);
}
