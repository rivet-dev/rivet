import { Combobox } from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { actorRegionsQueryOptions } from "../queries";
import { ActorRegion } from "./actors/actor-region";

interface RegionSelectProps {
	projectNameId: string;
	environmentNameId: string;
	onValueChange: (value: string) => void;
	value: string;
}

export function RegionSelect({
	projectNameId,
	environmentNameId,
	onValueChange,
	value,
}: RegionSelectProps) {
	const { data } = useSuspenseQuery(
		actorRegionsQueryOptions({
			projectNameId,
			environmentNameId,
		}),
	);

	const regions = [
		{
			label: "Automatic (Recommended)",
			value: "",
			region: { id: "", name: "Automatic (Recommended)" },
		},
		...data.map((region) => {
			return {
				label: (
					<ActorRegion
						projectNameId={projectNameId}
						environmentNameId={environmentNameId}
						regionId={region.id}
						showLabel
					/>
				),
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
