"use client";

import { Slot } from "@radix-ui/react-slot";
import {
	forwardRef,
	type MouseEventHandler,
	type PropsWithChildren,
} from "react";
import { toast } from "sonner";

export const CopyCodeTrigger = forwardRef<HTMLElement, PropsWithChildren>(
	({ children }, ref) => {
		const handleClick: MouseEventHandler = (event) => {
			const codeGroup = event.currentTarget.closest("[data-code-group]");

			const code =
				codeGroup?.querySelector("[data-state=active] .code")
					?.textContent ||
				codeGroup?.querySelector(".code")?.textContent;

			if (!code) {
				toast.error("No code to copy");
				return;
			}
			navigator.clipboard.writeText(code);
			toast.success("Copied to clipboard");
		};
		return <Slot onClick={handleClick}>{children}</Slot>;
	},
);
