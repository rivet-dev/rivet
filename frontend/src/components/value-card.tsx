import type { ReactNode } from "react";
import {
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
} from "./ui/card";

interface ValueCardProps {
	title: string;
	value: ReactNode;
	footer?: ReactNode;
}

export const ValueCard = ({ title, value, footer }: ValueCardProps) => {
	return (
		<Card className="flex flex-col">
			<CardHeader>
				<CardTitle className="text-sm font-medium">{title}</CardTitle>
			</CardHeader>
			<CardContent className="flex-1">
				<div className="text-4xl font-bold">{value}</div>
			</CardContent>
			{footer ? <CardFooter>{footer}</CardFooter> : null}
		</Card>
	);
};
