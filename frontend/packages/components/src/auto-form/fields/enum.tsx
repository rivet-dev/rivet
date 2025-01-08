import type * as z from "zod";
import { FormControl, FormItem, FormMessage } from "../../ui/form";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../ui/select";
import AutoFormLabel from "../common/label";
import AutoFormTooltip from "../common/tooltip";
import type { AutoFormInputComponentProps } from "../types";
import { getBaseSchema } from "../utils";

export default function AutoFormEnum({
	label,
	isRequired,
	field,
	fieldConfigItem,
	zodItem,
	fieldProps,
}: AutoFormInputComponentProps) {
	// biome-ignore lint/suspicious/noExplicitAny: FIXME
	const baseValues = (getBaseSchema(zodItem) as unknown as z.ZodEnum<any>)
		._def.values;

	let values: [string, string][] = [];
	if (!Array.isArray(baseValues)) {
		values = Object.entries(baseValues);
	} else {
		values = baseValues.map((value) => [value, value]);
	}

	// biome-ignore lint/suspicious/noExplicitAny: FIXME
	function findItem(value: any) {
		return values.find((item) => item[0] === value);
	}

	return (
		<FormItem>
			<AutoFormLabel
				label={fieldConfigItem?.label || label}
				isRequired={isRequired}
			/>
			<FormControl>
				<Select
					onValueChange={field.onChange}
					defaultValue={field.value}
					{...fieldProps}
				>
					<SelectTrigger className={fieldProps.className}>
						<SelectValue
							placeholder={
								fieldConfigItem.inputProps?.placeholder
							}
						>
							{field.value
								? findItem(field.value)?.[1]
								: "Select an option"}
						</SelectValue>
					</SelectTrigger>
					<SelectContent>
						{values.map(([value, label]) => (
							<SelectItem value={label} key={value}>
								{label}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
			</FormControl>
			<AutoFormTooltip fieldConfigItem={fieldConfigItem} />
			<FormMessage />
		</FormItem>
	);
}
