import { Combobox } from "@rivet-gg/components";
import { useAtomValue } from "jotai";
import { actorRegionsAtom } from "./actor-context";
import { ActorRegion } from "./actor-region";

interface RegionSelectProps {
	onValueChange: (value: string) => void;
	value: string;
}

export function RegionSelect({ onValueChange, value }: RegionSelectProps) {
	const data = useAtomValue(actorRegionsAtom);

	const regions = [
		{
			label: "Automatic (Recommended)",
			value: "",
			region: { id: "", name: "Automatic (Recommended)" },
		},
		...data.map((region) => {
			return {
				label: <ActorRegion regionId={region.id} showLabel />,
				value: region.id,
				region,
			};
		}),
	];

	return (
		<Combobox
			placeholder="Choose a region..."
			options={regions}
			value={value}
			onValueChange={onValueChange}
			filter={(option, searchMixed) => {
				const search = searchMixed.toLowerCase();
				return (
					option.region.id.includes(search) ||
					option.region.name.includes(search)
				);
			}}
			className="w-full"
		/>
	);
}
