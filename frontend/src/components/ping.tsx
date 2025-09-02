import type { ComponentPropsWithRef } from "react";
import { cn } from "./lib/utils";

type Variant = "primary" | "destructive" | "success";

const mainVariants = {
	primary: "bg-primary",
	success: "bg-green-500",
	destructive: "bg-red-500",
} satisfies Record<Variant, string>;

const pingVariants = {
	primary: "bg-primary/90",
	success: "bg-green-400",
	destructive: "bg-red-400",
} satisfies Record<Variant, string>;

interface PingProps extends ComponentPropsWithRef<"span"> {
	variant?: Variant;
	className?: string;
}

export const Ping = ({
	variant = "primary",
	className,
	...props
}: PingProps) => {
	return (
		<span
			{...props}
			className={cn("flex size-2 absolute top-0 -right-3", className)}
		>
			<span
				className={cn(
					"animate-ping absolute inline-flex h-full w-full rounded-full opacity-75 right-0",
					pingVariants[variant],
				)}
			/>
			<span
				className={cn(
					"relative inline-flex rounded-full size-2 bg-red-500",
					mainVariants[variant],
				)}
			/>
		</span>
	);
};
