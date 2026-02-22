"use client";

import { remToPx } from "@/lib/remToPx";
import { cn } from "@rivet-gg/components";
import { motion } from "framer-motion";
import { useCallback, useRef, useState } from "react";
import { useEffect } from "react";

const HEADER_HEIGHT = remToPx(6.5);
// const SCROLL_MARGIN = remToPx(9 /* scroll-mt-header-offset */ - HEADER_HEIGHT);
const LINK_MARGIN = remToPx(1);

function useScrollToActiveLink(currentSection) {
	const ref = useRef<HTMLDivElement>(null);

	// biome-ignore lint/correctness/useExhaustiveDependencies: this effect should only run when the currentSection changes
	useEffect(() => {
		const currentLink = ref.current?.querySelector(`[aria-current="page"]`);
		if (!currentLink || !ref.current) return;

		const linkRect = currentLink.getBoundingClientRect();
		const containerRect = ref.current.getBoundingClientRect();

		// calculate how much to scroll by
		// take into account the navigation header height
		const linkRelativeTop = linkRect.y - containerRect.top;

		// if the link is below the container, scroll down by the difference in height + the height of the link itself (so it's not at the bottom)
		if (linkRelativeTop + LINK_MARGIN >= containerRect.height) {
			// calculate the difference between the bottom of the link and the bottom of the container
			const bottomDifference =
				linkRelativeTop + LINK_MARGIN - containerRect.height + linkRect.height;
			ref.current.scrollBy(0, bottomDifference);
		}
		// if the link is above the container, scroll up by the difference in height + the height of the link itself (so it's not at the top)
		else if (linkRelativeTop < linkRect.height + LINK_MARGIN) {
			ref.current.scrollBy(0, linkRelativeTop - LINK_MARGIN);
		}
	}, [currentSection]);

	return ref;
}

function useCurrentSection(tableOfContents = []) {
	const [currentSection, setCurrentSection] = useState(
		tableOfContents?.[0]?.id || null,
	);
	const getHeadings = useCallback((tableOfContents) => {
		return tableOfContents
			.flatMap((node) => [node.id, ...node.children.map((child) => child.id)])
			.map((id) => {
				const el = document.getElementById(id);
				if (!el) return null;

				const style = window.getComputedStyle(el);
				const scrollMt = Number.parseFloat(style.scrollMarginTop);

				const top = window.scrollY + el.getBoundingClientRect().top - scrollMt;
				return { id, top };
			})
			.filter((x) => x !== null);
	}, []);

	useEffect(() => {
		if (!tableOfContents || tableOfContents?.length === 0) return;
		const headings = getHeadings(tableOfContents);
		if (headings.length === 0) return;
		function onScroll() {
			const top = window.scrollY;
			let current = headings[0].id;
			for (const heading of headings) {
				if (top >= heading.top - LINK_MARGIN) {
					current = heading.id;
				} else {
					break;
				}
			}
			setCurrentSection(current);
		}
		window.addEventListener("scroll", onScroll, { passive: true });
		onScroll();
		return () => {
			window.removeEventListener("scroll", onScroll);
		};
	}, [getHeadings, tableOfContents]);
	return currentSection;
}

function NavLink({ id, isActive, children }) {
	return (
		<>
			<a href={`#${id}`}
				aria-current={isActive ? "page" : undefined}
				className={cn(
					"group flex w-full items-center rounded-md border border-transparent px-2 py-1 text-sm text-muted-foreground hover:underline aria-current-page:text-foreground",
				)}
			>
				<span className="truncate">{children}</span>
			</a>
		</>
	);
}

export function ActiveSectionMarker({ prefix }) {
	return (
		<>
			<motion.div
				layout
				layoutId={`${prefix}current-background`}
				initial={{ opacity: 0 }}
				animate={{ opacity: 1 }}
				className="absolute inset-0 -left-2 bg-charcole-800/2.5 will-change-transform dark:bg-white/2.5"
				style={{ borderRadius: 8 }}
			/>
			<motion.div
				layout
				layoutId={`${prefix}current-line`}
				className="absolute left-0 top-1 h-6 w-px bg-cream-500"
				initial={{ opacity: 0 }}
				animate={{ opacity: 1 }}
			/>
		</>
	);
}

function Tree({ sections, isActive, depth = 0 }) {
	return (
		<>
			<ul>
				{sections.map((section) => {
					const isCurrentSectionActive = isActive(section);
					return (
						<li key={section.id} className="relative">
							<div className="relative">
								<NavLink
									key={section.id}
									id={section.id}
									isActive={isCurrentSectionActive}
								>
									{section.title}
								</NavLink>
							</div>

							{section.children && section.children.length > 0 ? (
								<div className="relative pl-3">
									<Tree
										sections={section.children}
										isActive={isActive}
										depth={depth + 1}
									/>
								</div>
							) : null}
						</li>
					);
				})}
			</ul>
		</>
	);
}

interface DocsTableOfContentsProps {
	// biome-ignore lint/suspicious/noExplicitAny: FIXME
	tableOfContents: any;
	className?: string;
}
export function DocsTableOfContents({
	tableOfContents: providedToc,
	className,
}: DocsTableOfContentsProps) {
	const tableOfContents = providedToc;

	const currentSection = useCurrentSection(tableOfContents);
	const ref = useScrollToActiveLink(currentSection);

	if (!tableOfContents || tableOfContents.length === 0) {
		return null;
	}

	function isActive(section) {
		if (section.id === currentSection) {
			return true;
		}
		if (!section.children) {
			return false;
		}
	}

	return (
		<div
			ref={ref}
			className={cn(
				className,
				"lg:top-header pt-10 lg:pt-6",
				"w-full lg:pointer-events-auto lg:sticky lg:block lg:max-w-aside lg:self-start lg:overflow-y-auto lg:max-h-content",
			)}
		>
			<div className="relative">
				<div className="relative">
					<Tree sections={tableOfContents} isActive={isActive} />
				</div>
			</div>
		</div>
	);
}
