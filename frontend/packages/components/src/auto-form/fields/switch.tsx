import { FormControl, FormItem } from "../../ui/form";
import { Switch } from "../../ui/switch";
import AutoFormLabel from "../common/label";
import AutoFormTooltip from "../common/tooltip";
import type { AutoFormInputComponentProps } from "../types";

export default function AutoFormSwitch({
	label,
	isRequired,
	field,
	fieldConfigItem,
	fieldProps,
}: AutoFormInputComponentProps) {
	return (
		<div>
			<FormItem>
				<div className="flex items-center gap-3">
					<FormControl>
						<Switch
							checked={field.value}
							onCheckedChange={field.onChange}
							{...fieldProps}
						/>
					</FormControl>
					<AutoFormLabel
						label={fieldConfigItem?.label || label}
						isRequired={isRequired}
					/>
				</div>
			</FormItem>
			<AutoFormTooltip fieldConfigItem={fieldConfigItem} />
		</div>
	);
}
