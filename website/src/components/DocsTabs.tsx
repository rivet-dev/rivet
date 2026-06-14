"use client";

import { usePathname } from "@/hooks/usePathname";
import { findPageForHref } from "@/lib/sitemap";
import { sitemap } from "@/sitemap/mod";
import { cn } from "@rivet-gg/components";

export function DocsTabs({
	light = false,
	initialPathname = "",
}: { light?: boolean; initialPathname?: string }) {
	// usePathname is empty during SSR and the first client render, so seed it
	// from the Astro-provided pathname. Otherwise no tab matches at SSR time and
	// the tab bar (which is not re-reconciled on the client) never highlights the
	// active tab.
	const pathname = usePathname() || initialPathname;
	// Remove trailing slash for consistency
	const normalizedPath = pathname.replace(/\/$/, "");

	return (
		<div className="-mx-8 hidden h-14 items-center gap-4 bg-[#e9e9eb] px-8 empty:hidden md:flex">
			{sitemap.map((tab) => {
				const isActive = findPageForHref(normalizedPath, tab);
				return (
					<a
						key={tab.href}
						href={tab.href}
						target={tab.target}
						aria-current={isActive ? "page" : undefined}
						className={cn(
							"px-0 text-sm hover:bg-transparent flex items-center border-b-2 border-transparent rounded-none h-full transition-colors",
							light
								? "text-ink-faint aria-[current=page]:text-ink aria-[current=page]:border-pine"
								: "text-muted-foreground aria-[current=page]:text-foreground aria-[current=page]:border-primary"
						)}
					>
						{tab.title}
						{tab.badge && (
							<span className={cn(
								"ml-1.5 whitespace-nowrap rounded-sm border px-[6px] py-0 text-[10px] font-medium",
								light ? "border-ink/10 bg-ink/[0.06] text-ink-soft" : "border-border bg-white/5 text-muted-foreground",
							)}>
								{tab.badge}
							</span>
						)}
					</a>
				);
			})}
		</div>
	);
}
