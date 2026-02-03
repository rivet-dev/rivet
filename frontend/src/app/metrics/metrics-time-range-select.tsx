import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { TIME_RANGE_OPTIONS } from "./constants";

interface MetricsTimeRangeSelectProps {
	value: string;
	onValueChange: (value: string) => void;
}

export function MetricsTimeRangeSelect({
	value,
	onValueChange,
}: MetricsTimeRangeSelectProps) {
	return (
		<Select value={value} onValueChange={onValueChange}>
			<SelectTrigger className="w-24">
				<SelectValue />
			</SelectTrigger>
			<SelectContent>
				{TIME_RANGE_OPTIONS.map((option) => (
					<SelectItem key={option.value} value={option.value}>
						{option.label}
					</SelectItem>
				))}
			</SelectContent>
		</Select>
	);
}
