"use client";

import * as TabsPrimitive from "@radix-ui/react-tabs";
import { motion } from "framer-motion";
import * as React from "react";

import { cn } from "../lib/utils";
import {
	type CommonHelperProps,
	getCommonHelperClass,
	omitCommonHelperProps,
} from "./helpers";

type TabsInstance = {
	activeValue: string | undefined;
	indicatorId: string;
};

const TabsContext = React.createContext<TabsInstance | null>(null);

const Tabs = React.forwardRef<
	React.ElementRef<typeof TabsPrimitive.Root>,
	React.ComponentPropsWithoutRef<typeof TabsPrimitive.Root> &
		Partial<CommonHelperProps>
>(
	(
		{ className, value, defaultValue, onValueChange, ...props },
		ref,
	) => {
		const rest = omitCommonHelperProps(props);
		const indicatorId = React.useId();
		const isControlled = value !== undefined;
		const [internalValue, setInternalValue] = React.useState<
			string | undefined
		>(defaultValue);
		const activeValue = isControlled ? value : internalValue;

		const handleValueChange = React.useCallback(
			(next: string) => {
				if (!isControlled) setInternalValue(next);
				onValueChange?.(next);
			},
			[isControlled, onValueChange],
		);

		const ctx = React.useMemo<TabsInstance>(
			() => ({ activeValue, indicatorId }),
			[activeValue, indicatorId],
		);

		return (
			<TabsContext.Provider value={ctx}>
				<TabsPrimitive.Root
					ref={ref}
					value={activeValue}
					defaultValue={isControlled ? undefined : defaultValue}
					onValueChange={handleValueChange}
					className={cn(className, getCommonHelperClass(props))}
					{...rest}
				/>
			</TabsContext.Provider>
		);
	},
);
Tabs.displayName = TabsPrimitive.Root.displayName;

const TabsList = React.forwardRef<
	React.ElementRef<typeof TabsPrimitive.List>,
	React.ComponentPropsWithoutRef<typeof TabsPrimitive.List>
>(({ className, ...props }, ref) => (
	<TabsPrimitive.List
		ref={ref}
		className={cn(
			"inline-flex text-muted-foreground border-b w-full",
			className,
		)}
		{...props}
	/>
));
TabsList.displayName = TabsPrimitive.List.displayName;

const TabsTrigger = React.forwardRef<
	React.ElementRef<typeof TabsPrimitive.Trigger>,
	React.ComponentPropsWithoutRef<typeof TabsPrimitive.Trigger>
>(({ className, children, ...props }, ref) => {
	const ctx = React.useContext(TabsContext);
	const isActive = ctx?.activeValue === props.value;
	return (
		<TabsPrimitive.Trigger
			ref={ref}
			className={cn(
				"inline-flex items-center justify-center whitespace-nowrap py-1 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 relative h-9 rounded-none bg-transparent px-4 pb-3 pt-2 font-semibold text-muted-foreground shadow-none transition-colors data-[state=active]:text-foreground",
				className,
			)}
			{...props}
		>
			{children}
			{isActive && ctx ? (
				<motion.span
					layoutId={ctx.indicatorId}
					className="absolute inset-x-0 -bottom-px h-[2px] bg-primary"
					transition={{
						type: "tween",
						duration: 0.18,
						ease: [0.32, 0.72, 0, 1],
					}}
				/>
			) : null}
		</TabsPrimitive.Trigger>
	);
});
TabsTrigger.displayName = TabsPrimitive.Trigger.displayName;

const TabsContent = React.forwardRef<
	React.ElementRef<typeof TabsPrimitive.Content>,
	React.ComponentPropsWithoutRef<typeof TabsPrimitive.Content>
>(({ className, ...props }, ref) => (
	<TabsPrimitive.Content
		ref={ref}
		className={cn(
			"mt-2 ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
			className,
		)}
		{...props}
	/>
));
TabsContent.displayName = TabsPrimitive.Content.displayName;

export { Tabs, TabsList, TabsTrigger, TabsContent };
