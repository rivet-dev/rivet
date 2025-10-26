import { faCalendar, Icon } from "@rivet-gg/icons";
import { format } from "date-fns";
import { forwardRef } from "react";
import { cn } from "../lib/utils";
import { Button } from "./button";
import { Calendar } from "./calendar";
import { Popover, PopoverContent, PopoverTrigger } from "./popover";

export const DatePicker = forwardRef<
	HTMLDivElement,
	{
		date?: Date;
		setDate: (date?: Date) => void;
	}
>(function DatePickerCmp({ date, setDate }, ref) {
	return (
		<Popover>
			<PopoverTrigger asChild>
				<Button
					variant={"outline"}
					className={cn(
						"w-full justify-start text-left font-normal",
						!date && "text-muted-foreground",
					)}
				>
					<Icon icon={faCalendar} className="mr-2 h-4 w-4" />
					{date ? format(date, "PPP") : <span>Pick a date</span>}
				</Button>
			</PopoverTrigger>
			<PopoverContent className="w-auto p-0" ref={ref}>
				<Calendar mode="single" selected={date} onSelect={setDate} />
			</PopoverContent>
		</Popover>
	);
});
