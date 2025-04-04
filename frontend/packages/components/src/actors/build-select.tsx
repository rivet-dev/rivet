import { Badge, Combobox } from "@rivet-gg/components";
import { useAtomValue } from "jotai";
import { actorBuildsAtom } from "./actor-context";
import { useMemo } from "react";

interface BuildSelectProps {
	onValueChange: (value: string) => void;
	value: string;
	onlyCurrent?: boolean;
}

export function BuildSelect({
	onValueChange,
	value,
	onlyCurrent,
}: BuildSelectProps) {
	const data = useAtomValue(actorBuildsAtom);

	const builds = useMemo(() => {
		let sorted = data.toSorted(
			(a, b) => b.createdAt.valueOf() - a.createdAt.valueOf(),
		);

		if (onlyCurrent) {
			sorted = sorted.filter((build) => build.tags.current);
		}

		const findLatest = (name: string) =>
			sorted.find((build) => build.tags.name === name);
		return sorted.map((build, index, array) => {
			return {
				label: (
					<div>
						<div className="flex flex-col gap-0.5 mb-1 text-left">
							<div className="font-semibold">
								{build.tags.name || build.id.split("-")[0]}

								{findLatest(build.tags.name)?.id ===
								build.id ? (
									<Badge className="ml-2" variant="outline">
										Latest
									</Badge>
								) : null}
							</div>
							<div className="text-xs">
								Created: {build.createdAt.toLocaleString()}
							</div>
						</div>
					</div>
				),
				value: build.id,
				build,
			};
		});
	}, [data, onlyCurrent]);

	return (
		<Combobox
			placeholder="Choose a build..."
			options={builds}
			value={value}
			onValueChange={onValueChange}
			filter={(option, search) =>
				option.build.name.includes(search) ||
				option.build.tags.name.includes(search)
			}
			className="w-full h-14"
		/>
	);
}
