import { DatePicker } from "../../ui/date-picker";
import { FormControl, FormItem, FormMessage } from "../../ui/form";
import AutoFormLabel from "../common/label";
import AutoFormTooltip from "../common/tooltip";
import type { AutoFormInputComponentProps } from "../types";

export default function AutoFormDate({
	label,
	isRequired,
	field,
	fieldConfigItem,
	fieldProps,
}: AutoFormInputComponentProps) {
	return (
		<FormItem>
			<AutoFormLabel
				label={fieldConfigItem?.label || label}
				isRequired={isRequired}
			/>
			<FormControl>
				<DatePicker
					date={field.value}
					setDate={field.onChange}
					{...fieldProps}
				/>
			</FormControl>
			<AutoFormTooltip fieldConfigItem={fieldConfigItem} />

			<FormMessage />
		</FormItem>
	);
}
