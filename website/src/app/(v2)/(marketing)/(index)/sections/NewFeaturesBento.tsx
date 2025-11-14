"use client";

import { useEffect } from "react";

export function NewFeaturesBento() {
	useEffect(() => {
		// Bento box glow effect
		const bentoBoxes = document.querySelectorAll<HTMLDivElement>(".bento-box");
		bentoBoxes.forEach((box) => {
			const handleMouseMove = (e: MouseEvent) => {
				const rect = box.getBoundingClientRect();
				const x = e.clientX - rect.left;
				const y = e.clientY - rect.top;
				box.style.setProperty("--mouse-x", `${x}px`);
				box.style.setProperty("--mouse-y", `${y}px`);
			};

			const handleMouseLeave = () => {
				box.style.setProperty("--mouse-x", "50%");
				box.style.setProperty("--mouse-y", "50%");
			};

			box.addEventListener("mousemove", handleMouseMove);
			box.addEventListener("mouseleave", handleMouseLeave);

			return () => {
				box.removeEventListener("mousemove", handleMouseMove);
				box.removeEventListener("mouseleave", handleMouseLeave);
			};
		});
	}, []);

	return (
		<section className="py-24 md:py-32">
			<h2 className="animate-on-scroll animate-fade-up text-center font-heading text-4xl md:text-5xl font-bold tracking-tighter text-text-primary">
				A New Primitive for Your Backend.
			</h2>

			<div className="mt-16 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
				<div className="bento-box animate-on-scroll animate-fade-up delay-100 border border-border rounded-xl p-6 md:p-8">
					<svg
						className="h-8 w-8 text-accent"
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						strokeWidth="1.5"
						stroke="currentColor"
					>
						<path
							strokeLinecap="round"
							strokeLinejoin="round"
							d="M3.75 3v11.25A2.25 2.25 0 0 0 6 16.5h12M3.75 3h-1.5m1.5 0h16.5m0 0h1.5m-1.5 0v11.25A2.25 2.25 0 0 1 18 16.5h-1.5m-12.75 0h1.5m11.25 0h1.5m-1.5 0v-1.5m0 1.5v1.5m0 0v1.5m0 0h1.5m-1.5 0h-1.5m0 0h-1.5m0 0h-1.5m9 3.75H9A2.25 2.25 0 0 1 6.75 18v-1.5M17.25 18v-1.5m0 1.5v-1.5m-10.5 0v1.5m0-1.5v1.5m0 0v-1.5m0 0h1.5m0 0H9"
						/>
					</svg>
					<h3 className="mt-4 font-heading text-xl font-bold text-text-primary">Long-Lived Compute</h3>
					<p className="mt-2 text-text-secondary">Like Lambda, but with memory. No 5-minute timeouts. No state loss.</p>
				</div>

				<div className="bento-box animate-on-scroll animate-fade-up delay-200 border border-border rounded-xl p-6 md:p-8">
					<svg
						className="h-8 w-8 text-accent"
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						strokeWidth="1.5"
						stroke="currentColor"
					>
						<path strokeLinecap="round" strokeLinejoin="round" d="M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z" />
					</svg>
					<h3 className="mt-4 font-heading text-xl font-bold text-text-primary">Zero-Latency State</h3>
					<p className="mt-2 text-text-secondary">
						State lives with your compute. Reads and writes are in-memory. No database round-trips.
					</p>
				</div>

				<div className="bento-box animate-on-scroll animate-fade-up delay-300 border border-border rounded-xl p-6 md:p-8">
					<svg
						className="h-8 w-8 text-accent"
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						strokeWidth="1.5"
						stroke="currentColor"
					>
						<path
							strokeLinecap="round"
							strokeLinejoin="round"
							d="M9 9.563C9 9.252 9.252 9 9.563 9h4.874c.311 0 .563.252.563.563v4.874c0 .311-.252.563-.563.563H9.564A.562.562 0 0 1 9 14.437V9.564ZM21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z"
						/>
					</svg>
					<h3 className="mt-4 font-heading text-xl font-bold text-text-primary">Realtime, Built-in</h3>
					<p className="mt-2 text-text-secondary">
						WebSockets and SSE out-of-the-box. Broadcast updates with one line of code.
					</p>
				</div>

				<div className="bento-box animate-on-scroll animate-fade-up delay-400 border border-border rounded-xl p-6 md:p-8">
					<svg
						className="h-8 w-8 text-accent"
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						strokeWidth="1.5"
						stroke="currentColor"
					>
						<path
							strokeLinecap="round"
							strokeLinejoin="round"
							d="M21.752 15.002A9.72 9.72 0 0 1 18 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 0 0 3 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 0 0 9.002-5.998Z"
						/>
					</svg>
					<h3 className="mt-4 font-heading text-xl font-bold text-text-primary">Sleep When Idle</h3>
					<p className="mt-2 text-text-secondary">
						Actors automatically hibernate to save costs and wake up instantly (zero cold start) on demand.
					</p>
				</div>

				<div className="bento-box animate-on-scroll animate-fade-up delay-500 border border-border rounded-xl p-6 md:p-8">
					<svg
						className="h-8 w-8 text-accent"
						role="img"
						viewBox="0 0 24 24"
						xmlns="http://www.w3.org/2000/svg"
						fill="currentColor"
					>
						<title>GitHub</title>
						<path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" />
					</svg>
					<h3 className="mt-4 font-heading text-xl font-bold text-text-primary">Open Source & Self-Hostable</h3>
					<p className="mt-2 text-text-secondary">
						No vendor lock-in, ever. Run on Rivet Cloud, Vercel, Railway, or your own bare metal.
					</p>
				</div>

				<div className="bento-box animate-on-scroll animate-fade-up delay-600 border border-border rounded-xl p-6 md:p-8">
					<svg
						className="h-8 w-8 text-accent"
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						strokeWidth="1.5"
						stroke="currentColor"
					>
						<path
							strokeLinecap="round"
							strokeLinejoin="round"
							d="M12 9v3.75m0-10.036A11.959 11.959 0 0 1 3.598 6 11.99 11.99 0 0 0 3 9.75c0 5.592 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.57-.598-3.75h-.152c-3.196 0-6.1-1.25-8.25-3.286Zm0 13.036h.008v.008H12v-.008Z"
						/>
					</svg>
					<h3 className="mt-4 font-heading text-xl font-bold text-text-primary">Resilient by Design</h3>
					<p className="mt-2 text-text-secondary">
						Built-in failover and automatic restarts preserve state integrity.
					</p>
				</div>
			</div>
		</section>
	);
}
