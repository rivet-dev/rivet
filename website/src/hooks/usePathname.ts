"use client";
import { useState, useEffect } from "react";

/**
 * Custom hook that replicates Next.js usePathname behavior
 * Returns the current pathname from window.location
 */
export function usePathname(): string {
	const [pathname, setPathname] = useState("");

	useEffect(() => {
		const update = () => setPathname(window.location.pathname);
		update();
		// Astro view transitions swap the page without remounting persisted
		// islands, so listen for navigation to keep the pathname current.
		document.addEventListener("astro:page-load", update);
		return () => document.removeEventListener("astro:page-load", update);
	}, []);

	return pathname;
}
