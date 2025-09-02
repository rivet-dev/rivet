import { Combobox } from "@/components";
import { CrashPolicy } from "./queries";

const VALUES = Array.from(Object.entries(CrashPolicy)).map(([key, value]) => ({
	label: key,
	value,
}));

interface CrashPolicySelectProps {
	onValueChange: (value: string) => void;
	value: string;
}

export function CrashPolicySelect({
	onValueChange,
	value,
}: CrashPolicySelectProps) {
	return (
		<Combobox
			placeholder="Choose a crash policy..."
			options={VALUES}
			value={value}
			onValueChange={onValueChange}
			className="w-full"
		/>
	);
}
