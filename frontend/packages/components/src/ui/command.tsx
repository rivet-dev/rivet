"use client";

import { type DialogProps, DialogTitle } from "@radix-ui/react-dialog";
import { VisuallyHidden } from "@radix-ui/react-visually-hidden";
import { faSearch, Icon } from "@rivet-gg/icons";
import { Command as CommandPrimitive } from "cmdk";
import * as React from "react";
import { cn } from "../lib/utils";
import { Dialog, DialogContent } from "./dialog";

const Command = React.forwardRef<
	React.ElementRef<typeof CommandPrimitive>,
	React.ComponentPropsWithoutRef<typeof CommandPrimitive>
>(({ className, ...props }, ref) => (
	<CommandPrimitive
		ref={ref}
		className={cn(
			"flex h-full w-full flex-col overflow-hidden rounded-md bg-popover text-popover-foreground",
			className,
		)}
		{...props}
	/>
));
Command.displayName = CommandPrimitive.displayName;

interface CommandDialogProps extends DialogProps {
	commandProps: React.ComponentProps<typeof Command>;
}

const CommandDialog = ({
	children,
	commandProps,
	...props
}: CommandDialogProps) => {
	return (
		<Dialog {...props}>
			<VisuallyHidden>
				<DialogTitle>Command panel dialog</DialogTitle>
			</VisuallyHidden>
			<DialogContent hideClose className="overflow-hidden p-0 shadow-lg">
				<Command
					{...commandProps}
					className="[&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-muted-foreground [&_[cmdk-group]:not([hidden])_~[cmdk-group]]:pt-0 [&_[cmdk-group]]:px-2 [&_[cmdk-input-wrapper]_svg]:h-4 [&_[cmdk-input-wrapper]_svg]:w-4 [&_[cmdk-input]]:h-12 [&_[cmdk-item]]:px-2 [&_[cmdk-item]]:py-3 [&_[cmdk-item]_svg]:size-4 [&_[cmdk-item]_svg]:mr-2"
				>
					{children}
				</Command>
			</DialogContent>
		</Dialog>
	);
};

const CommandInput = React.forwardRef<
	React.ElementRef<typeof CommandPrimitive.Input>,
	React.ComponentPropsWithoutRef<typeof CommandPrimitive.Input>
>(({ className, ...props }, ref) => (
	<div className="flex items-center border-b px-3" cmdk-input-wrapper="">
		<Icon icon={faSearch} className="mr-2 text-sm shrink-0" />
		<CommandPrimitive.Input
			ref={ref}
			className={cn(
				"flex h-11 w-full rounded-md bg-transparent py-3 text-sm outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50",
				className,
			)}
			{...props}
		/>
	</div>
));

CommandInput.displayName = CommandPrimitive.Input.displayName;

const CommandList = React.forwardRef<
	React.ElementRef<typeof CommandPrimitive.List>,
	React.ComponentPropsWithoutRef<typeof CommandPrimitive.List>
>(({ className, ...props }, ref) => (
	<CommandPrimitive.List
		ref={ref}
		className={cn(
			"max-h-[500px] overflow-y-auto overflow-x-hidden",
			className,
		)}
		{...props}
	/>
));

CommandList.displayName = CommandPrimitive.List.displayName;

const CommandEmpty = React.forwardRef<
	React.ElementRef<typeof CommandPrimitive.Empty>,
	React.ComponentPropsWithoutRef<typeof CommandPrimitive.Empty>
>((props, ref) => (
	<CommandPrimitive.Empty
		ref={ref}
		className="py-6 text-center text-sm"
		{...props}
	/>
));

CommandEmpty.displayName = CommandPrimitive.Empty.displayName;

const CommandGroup = React.forwardRef<
	React.ElementRef<typeof CommandPrimitive.Group>,
	React.ComponentPropsWithoutRef<typeof CommandPrimitive.Group>
>(({ className, ...props }, ref) => (
	<CommandPrimitive.Group
		ref={ref}
		className={cn(
			"overflow-hidden p-1 text-foreground [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-muted-foreground",
			className,
		)}
		{...props}
	/>
));

CommandGroup.displayName = CommandPrimitive.Group.displayName;

const CommandSeparator = React.forwardRef<
	React.ElementRef<typeof CommandPrimitive.Separator>,
	React.ComponentPropsWithoutRef<typeof CommandPrimitive.Separator>
>(({ className, ...props }, ref) => (
	<CommandPrimitive.Separator
		ref={ref}
		className={cn("-mx-1 my-1 h-px bg-border", className)}
		{...props}
	/>
));
CommandSeparator.displayName = CommandPrimitive.Separator.displayName;

const CommandItem = React.forwardRef<
	React.ElementRef<typeof CommandPrimitive.Item>,
	React.ComponentPropsWithoutRef<typeof CommandPrimitive.Item>
>(({ className, ...props }, ref) => (
	<CommandPrimitive.Item
		ref={ref}
		className={cn(
			"relative flex cursor-default select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none aria-selected:bg-accent aria-selected:text-foreground data-[disabled=true]:pointer-events-none data-[disabled=true]:opacity-50",
			className,
		)}
		{...props}
	/>
));

CommandItem.displayName = CommandPrimitive.Item.displayName;

const CommandShortcut = ({
	className,
	...props
}: React.HTMLAttributes<HTMLSpanElement>) => {
	return (
		<span
			className={cn(
				"ml-auto text-xs tracking-widest text-muted-foreground",
				className,
			)}
			{...props}
		/>
	);
};
CommandShortcut.displayName = "CommandShortcut";

const CommandLoading = React.forwardRef<
	React.ElementRef<typeof CommandPrimitive.Loading>,
	React.ComponentPropsWithoutRef<typeof CommandPrimitive.Loading>
>(({ ...props }, ref) => (
	<CommandPrimitive.Loading
		ref={ref}
		className="py-6 text-center text-sm"
		{...props}
	/>
));
CommandLoading.displayName = CommandPrimitive.displayName;

export {
	Command,
	CommandDialog,
	CommandInput,
	CommandList,
	CommandEmpty,
	CommandGroup,
	CommandItem,
	CommandShortcut,
	CommandSeparator,
	CommandLoading,
};
