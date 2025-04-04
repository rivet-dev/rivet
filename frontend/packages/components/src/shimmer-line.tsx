import { cn } from "./lib/utils";

interface ShimmerLineProps {
	className?: string;
}
export const ShimmerLine = ({ className }: ShimmerLineProps) => {
	return (
		<div
			className={cn(
				"animate-in fade-in absolute inset-x-0  w-full overflow-hidden",
				className,
			)}
		>
			<div className="animate-bounce-x from-secondary/0 via-primary to-secondary/0 relative -bottom-px h-1 bg-gradient-to-r" />
		</div>
	);
};
