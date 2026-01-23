"use client";

import { Slot } from "@radix-ui/react-slot";
import { faCopy, faEye, faEyeSlash, Icon } from "@rivet-gg/icons";
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

interface CopyTriggerProps extends ComponentProps<typeof Slot> {
	children: ReactNode;
	value: string | (() => string);
}

export const CopyTrigger = forwardRef<HTMLElement, CopyTriggerProps>(
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

export type DiscreteCopyButtonProps = CopyTriggerProps & {
	tooltip?: boolean;
} & Omit<ComponentProps<typeof Button>, "value">;

export const DiscreteCopyButton = forwardRef<
	HTMLElement,
	DiscreteCopyButtonProps
>(({ children, value, tooltip = true, ...props }, ref) => {
	const content = (
		<CopyTrigger ref={ref} value={value} {...props}>
			<Button
				type="button"
				variant="ghost"
				size={props.size}
				className={cn("max-w-full min-w-0", props.className)}
				endIcon={
					<Icon
						className="group-hover:opacity-100 opacity-0 transition-opacity"
						icon={faCopy}
					/>
				}
			>
				{children}
			</Button>
		</CopyTrigger>
	);

	if (tooltip) {
		return <WithTooltip content="Click to copy" trigger={content} />;
	}
	return content;
});

interface ClickToCopyProps
	extends Omit<ComponentProps<typeof WithTooltip>, "trigger" | "content"> {
	children: ReactNode;
	value: string;
}

export function ClickToCopy({ children, value, ...props }: ClickToCopyProps) {
	const handleClick = () => {
		navigator.clipboard.writeText(value);
		toast.success("Copied to clipboard");
	};
	return (
		<WithTooltip
			{...props}
			content="Click to copy"
			trigger={<Slot onClick={handleClick}>{children}</Slot>}
		/>
	);
}

export function DiscreteInput({
	value,
	show,
}: {
	value: string;
	show?: boolean;
}) {
	const [showState, setShowState] = useState(!!show);

	const finalShow = showState || !!show;
	return (
		<div className="relative">
			<Input
				type={finalShow ? "text" : "password"}
				readOnly
				value={value}
				className={cn("font-mono truncate", !show ? "pr-16" : "pr-8")}
			/>
			<div className="absolute right-1 top-1/2 -translate-y-1/2 opacity-50 flex gap-1">
				<ClickToCopy value={value} delayDuration={0}>
					<Button variant="ghost" size="icon-sm" type="button">
						<Icon icon={faCopy} />
					</Button>
				</ClickToCopy>
				{!show ? (
					<WithTooltip
						content={finalShow ? "Hide" : "Show"}
						trigger={
							<Button
								variant="ghost"
								size="icon-sm"
								type="button"
								onClick={() => setShowState(!showState)}
							>
								<Icon icon={showState ? faEye : faEyeSlash} />
							</Button>
						}
					/>
				) : null}
			</div>
		</div>
	);
}
