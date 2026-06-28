"use client";
import { createContext, useContext } from "react";
import type { ReactNode, AnchorHTMLAttributes } from "react";
import { normalizePath } from "@/lib/normalizePath";
import { usePathname } from "@/hooks/usePathname";

// SSR seed for path-derived state (active-link highlight + which collapsible
// sidebar section is open). Server-rendered islands have no `window`, and in
// `astro dev` docs islands don't hydrate at all — so without this seed the
// active section/link only resolves after hydration (or never, in dev). A
// parent (DocsNavigation) supplies the current path so it's correct at render
// time; the client `usePathname` effect then keeps it live across navigation.
export const SsrPathnameContext = createContext("");

export interface ActiveLinkProps extends AnchorHTMLAttributes<HTMLAnchorElement> {
	isActive?: boolean;
	children?: ReactNode;
	tree?: ReactNode;
	includeChildren?: boolean;
}

export function ActiveLink({
	isActive: isActiveOverride,
	tree,
	includeChildren,
	children,
	...props
}: ActiveLinkProps) {
	const pathname = usePathname(useContext(SsrPathnameContext));

	const isActive =
		isActiveOverride ||
		normalizePath(pathname) === normalizePath(String(props.href || "")) ||
		(includeChildren &&
			normalizePath(pathname).startsWith(
				normalizePath(String(props.href || "")),
			));
	return (
		<>
			<a {...props} aria-current={isActive ? "page" : undefined}>
				{children}
			</a>
			{isActive && tree ? tree : null}
		</>
	);
}
