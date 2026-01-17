import type { Rivet } from "@rivetkit/engine-api-full";
import { useInfiniteQuery } from "@tanstack/react-query";
import {
	type BaseComboboxProps,
	Combobox,
	type ComboboxMultipleProps,
	type ComboboxSingleProps,
} from "@/components";
import { ActorRegion } from "./actor-region";
import { useDataProvider } from "./data-provider";

type RegionSelectProps = {
	showAuto?: boolean;
} & Omit<
	BaseComboboxProps<{
		region: { id: string; name: string } | Rivet.Datacenter;
		label: React.ReactNode;
		value: string;
	}>,
	"options" | "filter"
> &
	(ComboboxSingleProps | ComboboxMultipleProps);
export function RegionSelect({ showAuto = true, ...props }: RegionSelectProps) {
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
						label: <span>Automatic</span>,
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
			{...props}
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
