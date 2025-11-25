"use client";

import { useEffect } from "react";

export function ScrollObserver({ children }: { children: React.ReactNode }) {
	useEffect(() => {
		const scrollElements = document.querySelectorAll(".animate-on-scroll");

		const observer = new IntersectionObserver(
			(entries) => {
				entries.forEach((entry) => {
					if (entry.isIntersecting) {
						entry.target.classList.add("is-visible");
						observer.unobserve(entry.target);
					}
				});
			},
			{ threshold: 0.1 }
		);

		scrollElements.forEach((el) => observer.observe(el));

		return () => {
			scrollElements.forEach((el) => observer.unobserve(el));
		};
	}, []);

	return <>{children}</>;
}
