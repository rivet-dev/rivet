"use client";
import type { ReactNode } from "react";
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from "./ui/card";

export interface ActionCardProps {
	title: string;
	description?: ReactNode;
	children?: ReactNode;
	action?: ReactNode;
	footer?: ReactNode;
	className?: string;
}

export const ActionCard = ({
	title,
	action,
	footer,
	description,
	className,
	children,
}: ActionCardProps) => {
	return (
		<Card className={className}>
			<CardHeader>
				<div className="flex flex-row items-center justify-between space-y-0 gap-2 pb-2">
					<CardTitle className="font-bold">{title}</CardTitle>
					{action}
				</div>
				{description ? (
					<CardDescription>{description}</CardDescription>
				) : null}
			</CardHeader>
			{children ? <CardContent>{children}</CardContent> : null}
			{footer ? <CardFooter>{footer}</CardFooter> : null}
		</Card>
	);
};
