"use client";
import Link, { type LinkProps } from "next/link";
import { usePathname } from "next/navigation";
import type { ReactNode } from "react";

export interface ActiveLinkProps<T> extends LinkProps<T> {
	isActive?: boolean;
	children?: ReactNode;
}

export function ActiveLink<T>({
	isActive: isActiveOverride,
	...props
}: ActiveLinkProps<T>) {
	const pathname = usePathname() || "";
	const isActive = isActiveOverride || pathname === props.href;
	return <Link<T> {...props} aria-current={isActive ? "page" : undefined} />;
}
