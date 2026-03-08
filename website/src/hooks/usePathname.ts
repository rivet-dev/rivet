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

		document.addEventListener("astro:after-swap", update);
		return () => document.removeEventListener("astro:after-swap", update);
	}, []);

	return pathname;
}
