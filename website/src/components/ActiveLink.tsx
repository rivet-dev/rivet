"use client";
import type { ReactNode, AnchorHTMLAttributes } from "react";
import { normalizePath } from "@/lib/normalizePath";
import { useState, useEffect } from "react";

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
	const [pathname, setPathname] = useState("");

	useEffect(() => {
		setPathname(window.location.pathname);
	}, []);

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
