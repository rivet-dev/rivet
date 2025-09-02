import { type ComponentProps, forwardRef } from "react";
import { cn } from "./lib/utils";

export const CodeMirrorContainer = forwardRef<
	HTMLDivElement,
	ComponentProps<"div">
>((props, ref) => {
	return (
		<div
			ref={ref}
			{...props}
			className={cn("border rounded-md overflow-hidden", props.className)}
		/>
	);
});
