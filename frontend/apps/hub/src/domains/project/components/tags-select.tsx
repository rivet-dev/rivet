import { Combobox } from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { actorBuildTagsQueryOptions } from "../queries";
import { ActorTag } from "./actors/actor-tags";

interface TagsSelectProps {
	projectId: string;
	environmentId: string;
	value: Record<string, string>;
	onValueChange: (value: Record<string, string>) => void;
}

export function TagsSelect({
	projectId,
	environmentId,
	value,
	onValueChange,
}: TagsSelectProps) {
	const { data } = useSuspenseQuery(
		actorBuildTagsQueryOptions({ projectId, environmentId }),
	);

	const tags = data.map((tag) => {
		return {
			label: (
				<ActorTag>
					<span>
						{tag.key}={tag.value}
					</span>
				</ActorTag>
			),
			value: `${tag.index}`,
			tag,
		};
	});

	const valArray = Object.entries(value);

	const val = valArray
		.map(([key, value]) => {
			const tag = data.find(
				(tag) => tag.key === key && tag.value === value,
			);
			return tag ? `${tag.index}` : null;
		})
		.filter((tag) => tag !== null) as string[];

	const handleValueChange = (value: string[]) => {
		const tags = data.filter((tag) => value.includes(tag.index));

		onValueChange(
			Object.fromEntries(tags.map((tag) => [tag.key, tag.value])),
		);
	};

	return (
		<Combobox
			multiple
			placeholder="Filter by tags..."
			options={tags}
			value={val}
			onValueChange={handleValueChange}
			className="w-full"
		/>
	);
}
