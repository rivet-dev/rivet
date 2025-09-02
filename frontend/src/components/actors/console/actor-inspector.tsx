import type { ComponentProps } from "react";
import { chromeDark, Inspector, type ObjectInspector } from "react-inspector";
import { cn } from "@/components";

const INSPECTOR_THEME = {
	...chromeDark,
	BASE_BACKGROUND_COLOR: "transparent",
};

export function ActorObjectInspector(
	props: ComponentProps<typeof ObjectInspector>,
) {
	return (
		<div
			className={cn(
				"break-words break-all whitespace-pre-wrap",
				props.className,
			)}
		>
			<Inspector
				{...props}
				table={false}
				data={props.data}
				// Invalid types for theme
				// @ts-ignore
				theme={INSPECTOR_THEME}
			/>
		</div>
	);
}
