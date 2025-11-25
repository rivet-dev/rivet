import type { ReactNode } from "react";
import { Header } from "@/components/v2/Header";

export default function LearnLayout({ children }: { children: ReactNode }) {
	return (
		<>
			<Header active="learn" variant="full-width" learnMode />

			<div className="learn-container">
				<div className="texture-overlay" />
				<div className="relative z-10">{children}</div>
			</div>
		</>
	);
}
