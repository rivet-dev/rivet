"use client";
import Link, { type LinkProps } from "next/link";
import { usePathname } from "next/navigation";
import type { ReactNode } from "react";
import { normalizePath } from "@/lib/normalizePath";

export interface ActiveLinkProps<T> extends LinkProps<T> {
	isActive?: boolean;
	children?: ReactNode;
	tree?: ReactNode;
	includeChildren?: boolean;
}

export function ActiveLink<T>({
	isActive: isActiveOverride,
	tree,
	includeChildren,
	...props
}: ActiveLinkProps<T>) {
	const pathname = usePathname() || "";

	const isActive =
		isActiveOverride ||
		normalizePath(pathname) === normalizePath(String(props.href)) ||
		(includeChildren &&
			normalizePath(pathname).startsWith(
				normalizePath(String(props.href)),
			));
	return (
		<>
			<Link<T> {...props} aria-current={isActive ? "page" : undefined} />
			{isActive && tree ? tree : null}
		</>
	);
}
