import { useInfiniteQuery } from "@tanstack/react-query";
import { Combobox } from "@/components";
import { ActorRegion } from "./actor-region";
import { useDataProvider } from "./data-provider";

interface RegionSelectProps {
	onValueChange: (value: string) => void;
	value: string | undefined;
	showAuto?: boolean;
}

export function RegionSelect({
	onValueChange,
	value,
	showAuto = true,
}: RegionSelectProps) {
	const {
		data = [],
		fetchNextPage,
		isLoading,
		isFetchingNextPage,
	} = useInfiniteQuery(useDataProvider().datacentersQueryOptions());

	const regions = [
		...(showAuto
			? [
					{
						label: <span>Automatic (Recommended)</span>,
						value: "auto",
						region: { id: "auto", name: "Automatic" },
					},
				]
			: []),
		...data.map((region) => {
			return {
				label: <ActorRegion regionId={region.name} showLabel />,
				value: region.name,
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
				return option.region.name.toLowerCase().includes(search);
			}}
			className="w-full"
		/>
	);
}
