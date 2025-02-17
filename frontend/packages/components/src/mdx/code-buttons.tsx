"use client";

import { Slot } from "@radix-ui/react-slot";
import type { MouseEventHandler, PropsWithChildren } from "react";
import { toast } from "sonner";

export function CopyCodeTrigger({ children }: PropsWithChildren) {
	const handleClick: MouseEventHandler = (event) => {
		const code =
			event.currentTarget.parentNode?.parentNode?.querySelector(
				".code",
			)?.innerText;
		navigator.clipboard.writeText(code);
		toast.success("Copied to clipboard");
	};
	return <Slot onClick={handleClick}>{children}</Slot>;
}
