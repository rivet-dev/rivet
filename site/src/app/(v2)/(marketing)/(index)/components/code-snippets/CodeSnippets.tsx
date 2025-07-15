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
		<div className="bg-white/2 border border-white/10 rounded-2xl overflow-hidden">
			<div className="py-2 border-b border-white/5">
				<h2 className="text-center text-white/40 text-600 text-sm font-medium">
					Examples
				</h2>
			</div>
			
			{/* Desktop view - hidden on small screens */}
			<div className="hidden sm:block">
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
		</div>
	);
}