"use client";
import { useSearchParams } from "next/navigation";
import { useEffect } from "react";

export function EmbedDetector() {
	const queryParams = useSearchParams();

	useEffect(() => {
		if (queryParams?.get("embed") === "true") {
			document.querySelector("body > header")?.classList.add("hidden");
		}
	}, [queryParams]);

	return null;
}
