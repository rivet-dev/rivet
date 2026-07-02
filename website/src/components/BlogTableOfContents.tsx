"use client";

import { remToPx } from "@/lib/remToPx";
import { cn } from "@rivet-gg/components";
import { useCallback, useEffect, useState } from "react";

// Table of contents for in-depth blog posts. Unlike the docs variant, this one
// never scrolls: it has no internal scroll container and does not auto-scroll
// the active link into view. It only highlights the current section as the
// reader moves through the page, and the whole rail is pinned by a sticky
// wrapper in BlogArticle.

const LINK_MARGIN = remToPx(1);

type TocNode = { id: string; title: string; children?: TocNode[] };

function flattenIds(toc: TocNode[]): string[] {
	return toc.flatMap((node) => [
		node.id,
		...(node.children ?? []).map((child) => child.id),
	]);
}

function useCurrentSection(toc: TocNode[]) {
	const [current, setCurrent] = useState<string | null>(toc?.[0]?.id ?? null);

	const getHeadings = useCallback((toc: TocNode[]) => {
		return flattenIds(toc)
			.map((id) => {
				const el = document.getElementById(id);
				if (!el) return null;
				const scrollMt = Number.parseFloat(
					window.getComputedStyle(el).scrollMarginTop,
				);
				return {
					id,
					top: window.scrollY + el.getBoundingClientRect().top - scrollMt,
				};
			})
			.filter((x): x is { id: string; top: number } => x !== null);
	}, []);

	useEffect(() => {
		if (!toc || toc.length === 0) return;
		const headings = getHeadings(toc);
		if (headings.length === 0) return;
		function onScroll() {
			const top = window.scrollY;
			let cur = headings[0].id;
			for (const heading of headings) {
				if (top >= heading.top - LINK_MARGIN) cur = heading.id;
				else break;
			}
			setCurrent(cur);
		}
		window.addEventListener("scroll", onScroll, { passive: true });
		onScroll();
		return () => window.removeEventListener("scroll", onScroll);
	}, [getHeadings, toc]);

	return current;
}

function TocLink({
	node,
	current,
	depth,
}: {
	node: TocNode;
	current: string | null;
	depth: number;
}) {
	const active = node.id === current;
	return (
		<a
			href={`#${node.id}`}
			aria-current={active ? "page" : undefined}
			className={cn(
				"block border-l py-1 transition-colors",
				depth === 0 ? "pl-3 text-sm" : "pl-6 text-[13px]",
				active
					? "border-pine text-ink"
					: "border-transparent text-ink-soft hover:text-ink",
			)}
		>
			<span className="block truncate">{node.title}</span>
		</a>
	);
}

interface BlogTableOfContentsProps {
	tableOfContents: TocNode[];
}

export function BlogTableOfContents({
	tableOfContents: toc,
}: BlogTableOfContentsProps) {
	const current = useCurrentSection(toc);

	if (!toc || toc.length === 0) return null;

	return (
		<ul>
			{toc.map((section) => (
				<li key={section.id}>
					<TocLink node={section} current={current} depth={0} />
					{section.children && section.children.length > 0 && (
						<ul>
							{section.children.map((child) => (
								<li key={child.id}>
									<TocLink node={child} current={current} depth={1} />
								</li>
							))}
						</ul>
					)}
				</li>
			))}
		</ul>
	);
}
