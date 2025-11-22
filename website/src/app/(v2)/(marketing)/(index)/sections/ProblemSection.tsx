"use client";

import { useEffect } from "react";

const problems = [
	{
		category: "Problem",
		number: "01",
		icon: (
			<svg xmlns="http://www.w3.org/2000/svg" className="h-6 w-6 text-red-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
				<circle cx="12" cy="12" r="10"></circle>
				<line x1="12" y1="8" x2="12" y2="12"></line>
				<line x1="12" y1="16" x2="12.01" y2="16"></line>
			</svg>
		),
		title: "The Old Way",
		description: "Your serverless function spins up. It queries a database. It does its job and dies.",
		code: "lambda()",
		status: "The pain",
		statusColor: "bg-red-500",
	},
	{
		category: "Complexity",
		number: "02",
		icon: (
			<svg xmlns="http://www.w3.org/2000/svg" className="h-6 w-6 text-yellow-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
				<path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path>
				<polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline>
				<line x1="12" y1="22.08" x2="12" y2="12"></line>
			</svg>
		),
		title: "The \"Glue Code\" Mess",
		description: "So you add Redis for state. And a message queue for jobs. And a WebSocket server for realtime. Your app is now a complex, distributed monolith.",
		code: "redis + queue + ws",
		status: "Complexity",
		statusColor: "bg-yellow-500",
	},
	{
		category: "Solution",
		number: "03",
		icon: (
			<svg xmlns="http://www.w3.org/2000/svg" className="h-6 w-6 text-green-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
				<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
				<polyline points="22 4 12 14.01 9 11.01"></polyline>
			</svg>
		),
		title: "The Rivet Way",
		description: "Your logic and state live together. One library. One process. It's fast, resilient, and scales from zero. No database round-trips. No queues. Just code.",
		code: "actor()",
		status: "The gain",
		statusColor: "bg-green-500",
	},
];

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
			<div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8">
				<h2 className="text-center font-heading text-4xl md:text-5xl font-bold tracking-tighter text-text-primary animate-on-scroll animate-fade-up mb-16">
					Your Stack is Stateless.
					<br />
					Your Apps Aren't.
				</h2>

				<div className="grid grid-cols-1 md:grid-cols-3 gap-6">
					{problems.map((problem, index) => {
						const delayClasses = ["delay-100", "delay-200", "delay-300"];
						const isSolution = problem.category === "Solution";
						const isComplexity = problem.category === "Complexity";
						const isProblem = problem.category === "Problem";
						return (
						<div key={index} className={`bento-box animate-on-scroll animate-fade-up ${delayClasses[index] || ""} border border-border rounded-xl p-6 md:p-8 bg-background/50 flex flex-col justify-between ${isSolution ? "ring-1 ring-green-500/30" : isComplexity ? "ring-1 ring-yellow-500/30" : isProblem ? "ring-1 ring-red-500/30" : ""}`}>
							{/* Header with category badge and number */}
							<div className="flex items-start justify-between gap-4 mb-6">
								<div className={`inline-flex bg-black/50 border border-border rounded-full px-3 py-1.5 items-center justify-center ${isSolution ? "border-green-500/30" : isComplexity ? "border-yellow-500/30" : isProblem ? "border-red-500/30" : ""}`}>
									<span className={`w-1.5 h-1.5 rounded-full ${isSolution ? "bg-green-500" : isComplexity ? "bg-yellow-500" : "bg-red-500"} mr-2`}></span>
									<span className="text-xs uppercase tracking-wider text-text-secondary">
										{problem.category}
									</span>
								</div>
								<span className="text-xs font-mono text-text-secondary">
									{problem.number}
								</span>
							</div>

							{/* Icon, title, and description */}
							<div className="flex-1">
								<div className="inline-flex items-center justify-center rounded-lg bg-black/30 border border-border p-3 mb-4">
									{problem.icon}
								</div>
								<h3 className={`font-heading text-xl font-bold mb-2 ${isSolution ? "text-green-500" : isComplexity ? "text-yellow-500" : "text-text-primary"}`}>
									{problem.title}
								</h3>
								<p className="text-text-secondary">
									{problem.description}
								</p>
							</div>
						</div>
						);
					})}
				</div>
			</div>
		</section>
	);
}
