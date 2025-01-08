import {
	Label,
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@rivet-gg/components";
import type { ComponentProps } from "react";

const sortOptions = [
	{ label: "by creation date (newest first)", value: "creation-date-newest" },
	{ label: "by creation date (oldest first)", value: "creation-date-oldest" },
	{ label: "by status", value: "status" },
	{ label: "by player count (biggest first)", value: "player-count-biggest" },
	{
		label: "by player count (smallest first)",
		value: "player-count-smallest",
	},
] as const;

export const SORT_VALUES = sortOptions.map((option) => option.value);

interface LobbySortSelectProps extends ComponentProps<typeof Select> {}

export function LobbySortSelect(props: LobbySortSelectProps) {
	return (
		<div className="flex gap-2 items-center">
			<Label>Sort</Label>
			<Select {...props}>
				<SelectTrigger>
					<SelectValue placeholder="Select team..." />
				</SelectTrigger>
				<SelectContent>
					{sortOptions.map((option) => (
						<SelectItem key={option.value} value={option.value}>
							{option.label}
						</SelectItem>
					))}
				</SelectContent>
			</Select>
		</div>
	);
}
