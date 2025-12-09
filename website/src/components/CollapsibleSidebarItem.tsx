"use client";

import type { SidebarItem, SidebarSection } from "@/lib/sitemap";
import { cn } from "@rivet-gg/components";
import { Icon, faChevronDown } from "@rivet-gg/icons";
import { motion } from "framer-motion";
import { usePathname } from "next/navigation";
import { type ReactNode, useMemo, useEffect, useState } from "react";
import { normalizePath } from "@/lib/normalizePath";
import { useNavigationState } from "@/providers/NavigationStateProvider";

interface CollapsibleSidebarItemProps {
	item: SidebarSection;
	children?: ReactNode;
	level?: number;
	parentPath?: string;
}

export function CollapsibleSidebarItem({
	item,
	children,
	level = 0,
	parentPath = "",
}: CollapsibleSidebarItemProps) {
	const pathname = usePathname() || "";
	const { isOpen, setIsOpen, toggleOpen } = useNavigationState();
	const hasActiveChild = findActiveItem(item.pages, pathname) !== null;
	const isCurrent = false; // Never highlight collapsible sections themselves
	const [shouldAnimate, setShouldAnimate] = useState(false);

	const itemId = useMemo(() => {
		return parentPath ? `${parentPath}.${item.title}` : item.title;
	}, [parentPath, item.title]);

	// Determine initial open state
	const getInitialState = () => {
		try {
			const savedStates = localStorage.getItem("rivet-navigation-state");
			if (savedStates) {
				const parsed = JSON.parse(savedStates);
				if (parsed.hasOwnProperty(itemId)) {
					return parsed[itemId];
				}
			}
		} catch (error) {
			// Ignore localStorage errors
		}
		// If no saved state, open if has active child
		return hasActiveChild;
	};

	const [isItemOpen, setIsItemOpen] = useState(getInitialState);

	// Sync with global state after mount
	// biome-ignore lint/correctness/useExhaustiveDependencies: it's okay, this runs only once
	useEffect(() => {
		const globalIsOpen = isOpen(itemId);
		if (globalIsOpen !== isItemOpen) {
			setIsOpen(itemId, isItemOpen);
		}
		// Enable animations after initial mount
		setShouldAnimate(true);
	}, []);

	// Update local state when global state changes
	useEffect(() => {
		const globalIsOpen = isOpen(itemId);
		if (globalIsOpen !== isItemOpen && shouldAnimate) {
			setIsItemOpen(globalIsOpen);
		}
	}, [isOpen, itemId, isItemOpen, shouldAnimate]);

	const getPaddingClass = (level: number) => {
		switch (level) {
			case 0:
				return "pl-2 pr-3";
			case 1:
				return "pl-5 pr-3";
			case 2:
				return "pl-8 pr-3";
			default:
				return "pl-11 pr-3";
		}
	};

	return (
		<div>
			<button
				type="button"
				className={cn(
					"flex w-full appearance-none items-center justify-between border-l-2 border-l-border py-2 text-sm text-muted-foreground transition-colors hover:text-foreground hover:border-l-muted-foreground/50 data-[active]:text-foreground data-[active]:border-l-orange-500",
					getPaddingClass(level),
				)}
				data-active={isCurrent ? true : undefined}
				onClick={() => {
					toggleOpen(itemId);
					setIsItemOpen(!isItemOpen);
				}}
			>
				<div className="flex items-center truncate gap-2">
					{item.icon ? (
						<Icon
							icon={item.icon}
							className="size-3.5 flex-shrink-0"
						/>
					) : null}
					<span className="truncate">{item.title}</span>
					{"badge" in item && item.badge ? (
						<span className="px-2 py-0.5 text-xs font-medium bg-muted text-muted-foreground rounded-full whitespace-nowrap">
							{item.badge}
						</span>
					) : null}
				</div>
				<motion.span
					variants={{
						open: { rotateZ: 0 },
						closed: { rotateZ: "-90deg" },
					}}
					initial={isItemOpen ? "open" : "closed"}
					animate={
						shouldAnimate ? (isItemOpen ? "open" : "closed") : false
					}
					className="ml-2 inline-block flex-shrink-0 opacity-70"
				>
					<Icon icon={faChevronDown} className="w-3 h-3" />
				</motion.span>
			</button>
			<motion.div
				className="overflow-hidden"
				variants={{
					open: { height: "auto", opacity: 1 },
					closed: { height: 0, opacity: 0 },
				}}
				initial={isItemOpen ? "open" : "closed"}
				animate={
					shouldAnimate ? (isItemOpen ? "open" : "closed") : false
				}
				transition={{
					opacity: isItemOpen ? { delay: 0.05 } : {},
					height: !isItemOpen ? { delay: 0.05 } : {},
					duration: 0.2,
				}}
			>
				{children}
			</motion.div>
		</div>
	);
}

function findActiveItem(pages: SidebarItem[], href: string) {
	for (const page of pages) {
		if (
			"href" in page &&
			normalizePath(page.href) === normalizePath(href)
		) {
			return page;
		}
		if ("pages" in page) {
			const found = findActiveItem(page.pages, href);
			if (found) {
				return found;
			}
		}
	}

	return null;
}
