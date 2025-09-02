"use client";

import { Slot } from "@radix-ui/react-slot";
import { faCopy, Icon } from "@rivet-gg/icons";
import {
	type ComponentProps,
	forwardRef,
	type MouseEventHandler,
	type ReactNode,
	useState,
} from "react";
import { toast } from "sonner";
import { cn } from "./lib/utils";
import { Button, type ButtonProps } from "./ui/button";
import { Flex } from "./ui/flex";
import { Input } from "./ui/input";
import { WithTooltip } from "./ui/tooltip";

interface CopyAreaProps {
	className?: string;
	value: string;
	display?: string;
	isConfidential?: boolean;
	variant?: "default" | "discrete";
	size?: ButtonProps["size"];
}

export const CopyArea = forwardRef<HTMLButtonElement, CopyAreaProps>(
	(
		{
			value,
			className,
			isConfidential,
			display,
			variant = "default",
			...props
		},
		ref,
	) => {
		const [isRevealed, setIsRevealed] = useState(false);
		const handleClick = () => {
			navigator.clipboard.writeText(value);
			toast.success("Copied to clipboard");
		};

		if (variant === "discrete") {
			return (
				<Button
					ref={ref}
					className={cn("font-mono", className)}
					variant="outline"
					type="button"
					endIcon={
						<Icon
							className="group-hover/button:opacity-100 opacity-0 transition-opacity"
							icon={faCopy}
						/>
					}
					{...props}
					onClick={handleClick}
				>
					<span className="flex-1 text-left truncate">
						{display || value}
					</span>
				</Button>
			);
		}

		return (
			<Flex gap="2" className={cn(className)} {...props}>
				{isConfidential ? (
					<WithTooltip
						content="Click to reveal"
						trigger={
							<Input
								readOnly
								value={display || value}
								onFocus={() => setIsRevealed(true)}
								onBlur={() => setIsRevealed(false)}
								className="font-mono"
								type={isRevealed ? "text" : "password"}
							/>
						}
					/>
				) : (
					<Input
						readOnly
						value={display || value}
						className="font-mono"
						type="text"
					/>
				)}

				<Button variant="secondary" size="icon" onClick={handleClick}>
					<Icon icon={faCopy} />
				</Button>
			</Flex>
		);
	},
);

interface CopyButtonProps extends ComponentProps<typeof Slot> {
	children: ReactNode;
	value: string | (() => string);
}

export const CopyButton = forwardRef<HTMLElement, CopyButtonProps>(
	({ children, value, ...props }, ref) => {
		const handleClick: MouseEventHandler<HTMLElement> = (event) => {
			event.stopPropagation();
			event.preventDefault();
			navigator.clipboard.writeText(
				typeof value === "function" ? value() : value,
			);
			toast.success("Copied to clipboard");
			props.onClick?.(event);
		};
		return (
			<Slot ref={ref} {...props} onClick={handleClick}>
				{children}
			</Slot>
		);
	},
);

export type DiscreteCopyButtonProps = CopyButtonProps &
	ComponentProps<typeof Button>;

export const DiscreteCopyButton = forwardRef<
	HTMLElement,
	DiscreteCopyButtonProps
>(({ children, value, ...props }, ref) => {
	return (
		<WithTooltip
			content="Click to copy"
			trigger={
				<CopyButton ref={ref} value={value} {...props}>
					<Button
						type="button"
						variant="ghost"
						size={props.size}
						className={cn(props.className, "max-w-full min-w-0")}
						endIcon={
							<Icon
								className="group-hover:opacity-100 opacity-0 transition-opacity"
								icon={faCopy}
							/>
						}
					>
						{children}
					</Button>
				</CopyButton>
			}
		/>
	);
});

interface ClickToCopyProps {
	children: ReactNode;
	value: string;
}

export function ClickToCopy({ children, value }: ClickToCopyProps) {
	const handleClick = () => {
		navigator.clipboard.writeText(value);
		toast.success("Copied to clipboard");
	};
	return (
		<WithTooltip
			content="Click to copy"
			trigger={<Slot onClick={handleClick}>{children}</Slot>}
		/>
	);
}
