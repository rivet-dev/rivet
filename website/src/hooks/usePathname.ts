"use client";
import { useState, useEffect } from "react";

/**
 * Custom hook that replicates Next.js usePathname behavior
 * Returns the current pathname from window.location
 */
export function usePathname(): string {
	const [pathname, setPathname] = useState("");

	useEffect(() => {
		setPathname(window.location.pathname);
	}, []);

	return pathname;
}
