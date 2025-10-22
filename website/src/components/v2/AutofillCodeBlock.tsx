"use client";

import { type ReactElement, useEffect, useMemo, useRef } from "react";
import { createRoot, type Root } from "react-dom/client";
import { TemplateVariable } from "@/components/v2/TemplateVariable";
import { useAutofillStore } from "@/stores/autofill-store";

interface AutofillCodeBlockProps {
	code: string;
	children: ReactElement;
}

export function AutofillCodeBlock({ code, children }: AutofillCodeBlockProps) {
	const codeRef = useRef<HTMLDivElement>(null);
	const rootsRef = useRef<Root[]>([]);
	const { getTemplateVariables } = useAutofillStore();

	// Calculate processed code for copy functionality
	const processedCode = useMemo(() => {
		const variables = getTemplateVariables();
		let result = code;

		for (const [key, value] of Object.entries(variables)) {
			// Match both simple and default value patterns
			const simpleRegex = new RegExp(`\\{\\{${key}\\}\\}`, "g");
			const defaultRegex = new RegExp(`\\{\\{${key}:[^}]+\\}\\}`, "g");

			result = result.replace(simpleRegex, value);
			result = result.replace(defaultRegex, value);
		}

		return result;
	}, [code, getTemplateVariables]);

	// Replace template variables in the DOM after render (only once)
	useEffect(() => {
		if (!codeRef.current) return;

		const codeElement = codeRef.current.querySelector(".code");
		if (!codeElement) return;

		// Check if already processed
		if (codeElement.hasAttribute("data-autofill-processed")) return;

		// Mark as processed to prevent re-processing
		codeElement.setAttribute("data-autofill-processed", "true");

		// Find all spans with data-template-var attribute (marked by Shiki transformer)
		const templateVarSpans = codeElement.querySelectorAll(
			"[data-template-var]",
		);

		templateVarSpans.forEach((span) => {
			const variable = span.getAttribute("data-template-var");
			const defaultValue = span.getAttribute("data-template-default");

			if (!variable) return;

			// Create a wrapper span for the React component
			const wrapper = document.createElement("span");
			wrapper.className = "template-variable-wrapper inline-block";

			// Mount React component
			const root = createRoot(wrapper);
			rootsRef.current.push(root);
			root.render(
				<TemplateVariable
					variable={variable}
					defaultValue={defaultValue || undefined}
				/>,
			);

			// Replace the original span with our wrapper
			span.parentNode?.replaceChild(wrapper, span);
		});

		// Cleanup on unmount
		return () => {
			rootsRef.current.forEach((root) => {
				try {
					root.unmount();
				} catch {
					// Ignore unmount errors
				}
			});
			rootsRef.current = [];
		};
	}, []);

	return (
		<div ref={codeRef} data-autofill-code={processedCode}>
			{children}
		</div>
	);
}
