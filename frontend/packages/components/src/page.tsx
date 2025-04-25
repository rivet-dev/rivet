import type { ReactNode } from "react";
import { cn } from "./lib/utils";
import { Flex } from "./ui/flex";
import { Skeleton } from "./ui/skeleton";
import { H1 } from "./ui/typography";

export interface PageProps {
	className?: string;
	title?: ReactNode;
	header?: ReactNode;
	layout?: "compact" | "full" | "onboarding" | "actors" | "v2";
	children: ReactNode;
}

export const Page = ({
	title,
	header,
	children,
	layout,
	className,
}: PageProps) => {
	return (
		<Flex
			direction="col"
			gap="4"
			className={cn(className, {
				"h-full":
					layout === "full" ||
					layout === "onboarding" ||
					layout === "actors" ||
					layout === "v2",
			})}
		>
			{title ? (
				<H1 className={cn(header ? "mt-8" : "my-8")}>{title}</H1>
			) : null}
			{header}
			{children}
		</Flex>
	);
};

Page.Skeleton = () => {
	return (
		<Flex direction="col" gap="4" w="full">
			<Skeleton className="my-4 h-12 w-1/3" />
			<div className="flex flex-row gap-4">
				<Skeleton className="h-64 w-2/3" />
				<Skeleton className="h-64 w-1/3" />
			</div>
			<Skeleton className="h-64 w-full" />
			<Skeleton className="h-64 w-full" />
		</Flex>
	);
};
