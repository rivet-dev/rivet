import * as React from "react";

import { cn } from "../lib/utils";

export interface TextareaProps
	extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {}

const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(
	({ className, ...props }, ref) => {
		return (
			<textarea
				className={cn(
					"flex min-h-[80px] w-full rounded-md border border-foreground/10 bg-foreground/[0.02] px-3 py-2 text-sm transition-colors placeholder:text-muted-foreground/60 hover:border-foreground/20 focus-visible:outline-none focus-visible:border-foreground/40 disabled:cursor-not-allowed disabled:opacity-50",
					className,
				)}
				ref={ref}
				{...props}
			/>
		);
	},
);
Textarea.displayName = "Textarea";

export { Textarea };
