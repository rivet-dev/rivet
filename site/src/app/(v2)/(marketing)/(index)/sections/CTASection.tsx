"use client";

import { MarketingButton } from "../components/MarketingButton";
import { useState, useEffect } from "react";
import { AnimatePresence, motion } from "framer-motion";

const CTA_TITLES = [
	"Performance in every act — thanks to Rivet Actors.",
	"Scale without drama — only with Rivet Actors.",
	"It's time your backend took center-stage — with Rivet Actors.",
	"SQLite the spotlight on performance — with Rivet Actors.",
	"Backend scalability: the SQL — starring Rivet Actors.",
	"Take your state to the edge — Rivet Actors makes it easy.",
	"No state fright — just scalability with Rivet Actors.",
	"Act now, deploy at the edge — with Rivet Actors.",
	"Lights, camera, serverless — powered by Rivet Actors.",
	"Your backend deserves a standing ovation — Rivet Actors delivers.",
	"Cue your backend's best performance — enter Rivet Actors.",
	"Backend performance worth applauding — only with Rivet Actors.",
	"Put your backend center-stage — with Rivet Actors.",
	"Make your backend the main actor — with Rivet Actors.",
	"Give your backend its big break — use Rivet Actors.",
	"Serverless, with no intermissions — powered by Rivet Actors.",
	"Set the stage for serverless success — with Rivet Actors."
];

export function CTASection() {
	const [currentIndex, setCurrentIndex] = useState(0);

	useEffect(() => {
		const interval = setInterval(() => {
			setCurrentIndex((prev) => (prev + 1) % CTA_TITLES.length);
		}, 3000);

		return () => clearInterval(interval);
	}, []);

	return (
		<div className="text-center mx-auto max-w-4xl">
			<h2 className="text-4xl sm:text-5xl font-700 text-white mb-4 min-h-[1.2em]">
				<AnimatePresence mode="wait">
					<motion.span
						key={currentIndex}
						initial={{ opacity: 0, y: 5 }}
						animate={{ opacity: 1, y: 0 }}
						exit={{ opacity: 0, y: -5 }}
						transition={{ duration: 0.1 }}
						style={{ display: "block" }}
					>
						{CTA_TITLES[currentIndex]}
					</motion.span>
				</AnimatePresence>
			</h2>

			<div className="h-8" />

			<div className="flex flex-col sm:flex-row gap-4 justify-center mb-4">
				<MarketingButton href="/docs/actors" primary>
					Get Started
				</MarketingButton>
				<MarketingButton href="https://github.com/rivet-gg/rivetkit/tree/main/examples">
					See Examples
				</MarketingButton>
			</div>
		</div>
	);
}
