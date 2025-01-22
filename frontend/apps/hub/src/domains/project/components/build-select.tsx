import { Combobox } from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { actorBuildsQueryOptions } from "../queries";

interface BuildSelectProps {
	projectNameId: string;
	environmentNameId: string;
	onValueChange: (value: string) => void;
	value: string;
}

export function BuildSelect({
	projectNameId,
	environmentNameId,
	onValueChange,
	value,
}: BuildSelectProps) {
	const { data } = useSuspenseQuery(
		actorBuildsQueryOptions({
			projectNameId,
			environmentNameId,
			tags: { current: "true" },
		}),
	);

	const builds = data
		.toSorted((a, b) => b.createdAt.valueOf() - a.createdAt.valueOf())
		.map((build) => {
			return {
				label: (
					<div>
						<div className="flex flex-col gap-0.5 mb-1 text-left">
							<div className="font-semibold">
								{build.tags.name || build.id.split("-")[0]}
							</div>
							<div className="text-xs">
								Created: {build.createdAt.toLocaleDateString()}
							</div>
						</div>
					</div>
				),
				value: build.id,
				build,
			};
		});

	return (
		<Combobox
			placeholder="Choose a build..."
			options={builds}
			value={value}
			onValueChange={onValueChange}
			filter={(option, search) =>
				option.build.id.includes(search) ||
				option.build.name.includes(search)
			}
			className="w-full"
		/>
	);
}
