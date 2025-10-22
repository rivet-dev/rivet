"use client";

import { useState } from "react";
import { examples, type StateTypeTab } from "@/data/examples/examples";
import CodeSnippetsDesktop from "./CodeSnippetsDesktop";
import CodeSnippetsMobile from "./CodeSnippetsMobile";

export default function CodeSnippets() {
	const [activeExample, setActiveExample] = useState<string>(examples[0].id);
	const [activeStateType, setActiveStateType] =
		useState<StateTypeTab>("memory");

	return (
		<>
			{/* Desktop view - hidden on small screens */}
			<div className="hidden sm:block bg-white/[0.01] border border-white/15 rounded-2xl overflow-hidden">
				<CodeSnippetsDesktop
					activeExample={activeExample}
					setActiveExample={setActiveExample}
					activeStateType={activeStateType}
					setActiveStateType={setActiveStateType}
				/>
			</div>

			{/* Mobile view - shown only on small screens */}
			<div className="block sm:hidden">
				<CodeSnippetsMobile />
			</div>
		</>
	);
}
