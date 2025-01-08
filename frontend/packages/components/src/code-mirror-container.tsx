import { type ComponentProps, forwardRef } from "react";

export const CodeMirrorContainer = forwardRef<
	HTMLDivElement,
	ComponentProps<"div">
>((props, ref) => {
	return (
		<div
			ref={ref}
			{...props}
			className="border rounded-md overflow-hidden"
		/>
	);
});
