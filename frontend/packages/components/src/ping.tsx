import { cn } from "./lib/utils";

type Variant = "primary" | "destructive";

const mainVariants = {
	primary: "bg-primary",
	destructive: "bg-red-500",
} satisfies Record<Variant, string>;

const pingVariants = {
	primary: "bg-primary/90",
	destructive: "bg-red-400",
} satisfies Record<Variant, string>;

interface PingProps {
	variant?: Variant;
	className?: string;
}

export const Ping = ({ variant = "primary", className }: PingProps) => {
	return (
		<span className={cn("flex size-2 absolute top-0 -right-3", className)}>
			<span
				className={cn(
					"animate-ping absolute inline-flex h-full w-full rounded-full opacity-75",
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
