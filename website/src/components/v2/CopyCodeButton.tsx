"use client";
import { Slot, toast } from "@rivet-gg/components";

export function CopyCodeTrigger({ children }) {
	const handleClick = (event) => {
		// Check if this is an autofill code block with processed code
		const autofillContainer = event.currentTarget.closest('[data-autofill-code]');
		let code: string;

		if (autofillContainer) {
			// Use the autofilled code from the data attribute
			code = autofillContainer.getAttribute('data-autofill-code') || '';
		} else {
			// Use the original behavior - get code from innerText
			code =
				event.currentTarget.closest('[data-code-block]')?.querySelector(
					".code",
				)?.innerText ?? '';
		}

		navigator.clipboard.writeText(code);
		toast.success("Copied to clipboard");
	};
	return <Slot onClick={handleClick}>{children}</Slot>;
}
