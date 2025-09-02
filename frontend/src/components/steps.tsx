import { cn } from "./lib/utils";

interface StepsProps {
	className?: string;
	children: React.ReactNode;
}

export function Steps({ className, children }: StepsProps) {
	return (
		<div
			className={cn(
				"[&>h3]:step steps mb-12 ml-4 border-l pl-8 [counter-reset:step]",
				className,
			)}
		>
			{children}
		</div>
	);
}

interface StepProps {
	children: React.ReactNode;
	className?: string;
	title: string;
}

export function Step({ children, className, title }: StepProps) {
	return (
		<>
			<h3
				className={cn(
					"font-heading mt-8 scroll-m-20 text-xl font-semibold tracking-tight",
					className,
				)}
			>
				{title}
			</h3>
			{children}
		</>
	);
}
