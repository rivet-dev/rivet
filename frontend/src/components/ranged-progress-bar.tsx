import type { ReactNode } from "react";
import { cn } from "./lib/utils";
import { Progress } from "./ui/progress";

const CSS_PROPERTY_MIN_LABEL_NAME = "--rvt-progress-min-label-x";
const CSS_PROPERTY_MAX_LABEL_NAME = "--rvt-progress-max-label-x";

type TailwindLabelTranslateClass = `${string}-${string}-[${
	| typeof CSS_PROPERTY_MIN_LABEL_NAME
	| typeof CSS_PROPERTY_MAX_LABEL_NAME}]`;

const MIN_LABEL_TRANSLATE_CLASS: TailwindLabelTranslateClass =
	"translate-x-[--rvt-progress-min-label-x]";

const MAX_LABEL_TRANSLATE_CLASS: TailwindLabelTranslateClass =
	"translate-x-[--rvt-progress-max-label-x]";

interface RangedProgressBarProps {
	percentage: number;
	maxRange: number;
	minLabel: ReactNode;
	maxLabel: ReactNode;
}

export function RangedProgressBar({
	percentage,
	maxRange,
	minLabel,
	maxLabel,
}: RangedProgressBarProps) {
	return (
		<div
			className="w-full relative overflow-hidden h-40"
			style={{
				[CSS_PROPERTY_MIN_LABEL_NAME as string]: `${percentage}%`,
				[CSS_PROPERTY_MAX_LABEL_NAME as string]: `${maxRange}%`,
			}}
		>
			<Progress value={percentage} className="top-1/2" />
			<span
				className={cn(
					"border-l pl-4 pt-8 pb-1 border-white w-full top-1/2 absolute -translate-y-1",
					MIN_LABEL_TRANSLATE_CLASS,
				)}
			>
				{minLabel}
			</span>
			<span
				className={cn(
					"border-l pl-4 pb-8 pt-1 border-white translate-x-[75%] w-full top-4 absolute",
					MAX_LABEL_TRANSLATE_CLASS,
				)}
			>
				{maxLabel}
			</span>
		</div>
	);
}
