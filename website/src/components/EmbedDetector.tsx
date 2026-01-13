"use client";
import { useEffect } from "react";

export function EmbedDetector() {
	useEffect(() => {
		const queryParams = new URLSearchParams(window.location.search);
		if (queryParams.get("embed") === "true") {
			document.querySelector("body > header")?.classList.add("hidden");
		}
	}, []);

	return null;
}
