"use client";

import * as SeparatorPrimitive from "@radix-ui/react-separator";
import * as React from "react";

import { cn } from "../lib/utils";
import {
	type CommonHelperProps,
	getCommonHelperClass,
	omitCommonHelperProps,
} from "./helpers";

const Separator = React.forwardRef<
	React.ElementRef<typeof SeparatorPrimitive.Root>,
	React.ComponentPropsWithoutRef<typeof SeparatorPrimitive.Root> &
		Partial<CommonHelperProps>
>(
	(
		{ className, orientation = "horizontal", decorative = true, ...props },
		ref,
	) => (
		<SeparatorPrimitive.Root
			ref={ref}
			decorative={decorative}
			orientation={orientation}
			className={cn(
				"shrink-0 bg-border",
				orientation === "horizontal"
					? "h-[1px] w-full"
					: "h-full w-[1px]",
				getCommonHelperClass(props),
				className,
			)}
			{...omitCommonHelperProps(props)}
		/>
	),
);
Separator.displayName = SeparatorPrimitive.Root.displayName;

export { Separator };
