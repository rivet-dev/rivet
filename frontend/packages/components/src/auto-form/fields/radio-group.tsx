import type * as z from "zod";
import { FormControl, FormItem, FormLabel, FormMessage } from "../../ui/form";
import { RadioGroup, RadioGroupItem } from "../../ui/radio-group";
import AutoFormLabel from "../common/label";
import AutoFormTooltip from "../common/tooltip";
import type { AutoFormInputComponentProps } from "../types";
import { getBaseSchema } from "../utils";

export default function AutoFormRadioGroup({
	label,
	isRequired,
	field,
	zodItem,
	fieldProps,
	fieldConfigItem,
}: AutoFormInputComponentProps) {
	// biome-ignore lint/suspicious/noExplicitAny: TODO: Fix this
	const baseValues = (getBaseSchema(zodItem) as unknown as z.ZodEnum<any>)
		._def.values;

	let values: string[] = [];
	if (!Array.isArray(baseValues)) {
		values = Object.entries(baseValues).map((item) => item[0]);
	} else {
		values = baseValues;
	}

	return (
		<div>
			<FormItem>
				<AutoFormLabel
					label={fieldConfigItem?.label || label}
					isRequired={isRequired}
				/>
				<FormControl>
					<RadioGroup
						onValueChange={field.onChange}
						defaultValue={field.value}
						{...fieldProps}
					>
						{
							/* biome-ignore lint/suspicious/noExplicitAny: TODO: Fix this */
							values?.map((value: any) => (
								<FormItem
									key={value}
									className="mb-2 flex items-center gap-3 space-y-0"
								>
									<FormControl>
										<RadioGroupItem value={value} />
									</FormControl>
									<FormLabel className="font-normal">
										{value}
									</FormLabel>
								</FormItem>
							))
						}
					</RadioGroup>
				</FormControl>
				<FormMessage />
			</FormItem>
			<AutoFormTooltip fieldConfigItem={fieldConfigItem} />
		</div>
	);
}
