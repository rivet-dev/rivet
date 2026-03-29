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
						{tab.badge && (
							<span className="ml-1.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground border border-border px-1 py-px rounded">
								{tab.badge}
							</span>
						)}
					</a>
				);
			})}
		</div>
	);
}
