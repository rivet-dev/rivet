import { useInfiniteQuery } from "@tanstack/react-query";
import { Combobox } from "@/components";
import { ActorRegion } from "./actor-region";
import { useManager } from "./manager-context";

interface RegionSelectProps {
	onValueChange: (value: string) => void;
	value: string;
}

export function RegionSelect({ onValueChange, value }: RegionSelectProps) {
	const {
		data = [],
		fetchNextPage,
		isLoading,
		isFetchingNextPage,
	} = useInfiniteQuery(useManager().regionsQueryOptions());

	const regions = [
		{
			label: <span>Automatic (Recommended)</span>,
			value: "auto",
			region: { id: "auto", name: "Automatic (Recommended)" },
		},
		...data.map((region) => {
			return {
				label:
					region.name === "local" ? (
						<ActorRegion regionId={region.id} showLabel />
					) : (
						region.name
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
			isLoading={isLoading || isFetchingNextPage}
			onLoadMore={fetchNextPage}
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
