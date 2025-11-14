"use client";

import { useEffect } from "react";

export function ProblemSection() {
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
				Your Stack is Stateless.
				<br />
				Your Apps Aren't.
			</h2>

			<div className="mt-16 grid grid-cols-1 md:grid-cols-3 gap-8">
				<div className="bento-box animate-on-scroll animate-fade-up delay-100 border border-border rounded-xl p-6 md:p-8 bg-background/50">
					<h3 className="font-heading text-xl font-bold text-text-primary">The Old Way</h3>
					<div className="mt-4 space-y-3 text-text-secondary">
						<p>Your serverless function spins up.</p>
						<p>It queries a database.</p>
						<p>It does its job and dies.</p>
					</div>
					<div className="mt-4 pt-4 border-t border-border">
						<p className="text-text-secondary">
							<span className="font-medium text-red-500">The pain:</span> Latency. Complexity. Cost.
						</p>
					</div>
				</div>

				<div className="bento-box animate-on-scroll animate-fade-up delay-200 border border-border rounded-xl p-6 md:p-8 bg-background/50">
					<h3 className="font-heading text-xl font-bold text-text-primary">The "Glue Code" Mess</h3>
					<div className="mt-4 space-y-3 text-text-secondary">
						<p>So you add Redis for state. And a message queue for jobs. And a WebSocket server for realtime.</p>
						<p>Your app is now a complex, distributed monolith.</p>
					</div>
				</div>

				<div className="bento-box animate-on-scroll animate-fade-up delay-300 border border-border rounded-xl p-6 md:p-8 bg-background/50 ring-1 ring-accent/30">
					<h3 className="font-heading text-xl font-bold text-accent">The Rivet Way</h3>
					<div className="mt-4 space-y-3 text-text-secondary">
						<p>Your logic and state live together. One library. One process. It's fast, resilient, and scales from zero.</p>
						<p>No database round-trips. No queues. Just code.</p>
					</div>
					<div className="mt-4 pt-4 border-t border-border">
						<p className="text-text-secondary">
							<span className="font-medium text-green-500">The gain:</span> Simplicity. Speed. Power.
						</p>
					</div>
				</div>
			</div>
		</section>
	);
}
