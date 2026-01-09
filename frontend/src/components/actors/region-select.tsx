import { useInfiniteQuery } from "@tanstack/react-query";
import { Combobox } from "@/components";
import { ActorRegion } from "./actor-region";
import { useDataProvider } from "./data-provider";

type RegionSelectProps = {
	showAuto?: boolean;
} & (
	| {
			onValueChange: (value: string) => void;
			value: string | undefined;
			multiple?: false;
	  }
	| {
			onValueChange: (value: string[]) => void;
			value: string[] | undefined;
			multiple: true;
	  }
);

export function RegionSelect({ showAuto = true, ...props }: RegionSelectProps) {
	const {
		data = [],
		fetchNextPage,
		isLoading,
		isFetchingNextPage,
	} = useInfiniteQuery(useDataProvider().regionsQueryOptions());

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
			{...props}
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
