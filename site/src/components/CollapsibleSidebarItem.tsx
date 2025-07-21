"use client";

import type { SidebarItem, SidebarSection } from "@/lib/sitemap";
import { cn } from "@rivet-gg/components";
import { Icon, faChevronDown } from "@rivet-gg/icons";
import { motion } from "framer-motion";
import { usePathname } from "next/navigation";
import { type ReactNode, useMemo, useEffect } from "react";
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
	
	const itemId = useMemo(() => {
		return parentPath ? `${parentPath}.${item.title}` : item.title;
	}, [parentPath, item.title]);
	
	const isItemOpen = isOpen(itemId);
	
	// Initialize state based on whether this section contains the active page
	// but only if no saved state exists
	useEffect(() => {
		try {
			const savedStates = localStorage.getItem("rivet-navigation-state");
			const hasNoSavedState = !savedStates || !JSON.parse(savedStates).hasOwnProperty(itemId);
			
			if (hasActiveChild && hasNoSavedState) {
				setIsOpen(itemId, true);
			}
		} catch (error) {
			// If localStorage is not available or parsing fails, fall back to default behavior
			if (hasActiveChild) {
				setIsOpen(itemId, true);
			}
		}
	}, [itemId, hasActiveChild, setIsOpen]);
	
	const getPaddingClass = (level: number) => {
		switch (level) {
			case 0: return "pl-3 pr-3";
			case 1: return "pl-6 pr-3";
			case 2: return "pl-9 pr-3";
			default: return "pl-12 pr-3";
		}
	};
	
	return (
		<div>
			<button
				type="button"
				className={cn(
					"flex w-full appearance-none items-center justify-between border-l-2 border-l-border py-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground hover:border-l-muted-foreground/50 data-[active]:text-foreground data-[active]:border-l-orange-500",
					getPaddingClass(level),
				)}
				data-active={isCurrent ? true : undefined}
				onClick={() => toggleOpen(itemId)}
			>
				<div className="flex items-center truncate">
					{item.icon ? (
						<Icon
							icon={item.icon}
							className="mr-2 size-3.5 flex-shrink-0"
						/>
					) : null}
					<span className="truncate">{item.title}</span>
				</div>
				<motion.span
					variants={{
						open: { rotateZ: 0 },
						closed: { rotateZ: "-90deg" },
					}}
					animate={isItemOpen ? "open" : "closed"}
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
				animate={isItemOpen ? "open" : "closed"}
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
		if ("href" in page && normalizePath(page.href) === normalizePath(href)) {
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
