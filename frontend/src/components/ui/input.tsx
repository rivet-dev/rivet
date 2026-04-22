import { Slot } from "@radix-ui/react-slot";
import { forwardRef, type ReactNode } from "react";
import { cn } from "../lib/utils";

export interface InputProps extends React.ComponentPropsWithoutRef<"input"> {
	asChild?: boolean;
	children?: ReactNode;
	className?: string;
}

const Input = forwardRef<HTMLInputElement, InputProps>(
	({ asChild, className, ...other }, ref) => {
		const Comp = asChild ? Slot : "input";
		return (
			<Comp
				ref={ref}
				{...other}
				className={cn(
					"flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm transition-colors file:border-0 file:text-foreground file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground/50 focus-visible:outline-none focus-visible:border-foreground/40 focus-within:border-foreground/40 disabled:cursor-not-allowed disabled:opacity-50",
					className,
				)}
			/>
		);
	},
);

Input.displayName = "Input";

export { Input };
