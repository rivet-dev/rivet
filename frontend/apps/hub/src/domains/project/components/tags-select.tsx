import { Combobox } from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { actorBuildTagsQueryOptions } from "../queries";
import { ActorTag } from "./actors/actor-tags";

interface TagsSelectProps {
	projectId: string;
	environmentId: string;
	value: Record<string, string>;
	onValueChange: (value: Record<string, string>) => void;
	showSelectedOptions?: number;
}

export function TagsSelect({
	projectId,
	environmentId,
	value,
	onValueChange,
	showSelectedOptions,
}: TagsSelectProps) {
	const { data } = useSuspenseQuery(
		actorBuildTagsQueryOptions({ projectId, environmentId }),
	);

	const valArray = Object.entries(value);

	// upsert custom tags to the list of tags
	const tags = [...data];
	for (const [key, value] of valArray) {
		const found = data.find(
			(tag) => tag.key === key && tag.value === value,
		);

		if (!found) {
			tags.push({ key, value });
		}
	}

	const options = tags.map((tag) => {
		return {
			label: (
				<ActorTag>
					<span>
						{tag.key}={tag.value}
					</span>
				</ActorTag>
			),
			value: [tag.key, tag.value].join("="),
			tag,
		};
	});

	const val = valArray.map(([key, value]) => {
		return [key, value].join("=");
	});

	const handleValueChange = (value: string[]) => {
		onValueChange(
			Object.fromEntries(
				value.map((v) => {
					// its safe to split by "=" because the value is a tag
					const [key, value] = v.split("=");
					return [key.trim(), value.trim()];
				}),
			),
		);
	};

	const handleCreateOption = (option: string) => {
		const parts = option.split("=");
		if (parts.length !== 2) return;

		const [key, value] = parts.map((part) => part.trim());
		if (!key || !value) return;

		onValueChange(Object.fromEntries([...valArray, [key, value]]));
	};

	return (
		<Combobox
			multiple
			placeholder="Filter by tags..."
			allowCreate
			onCreateOption={handleCreateOption}
			options={options}
			value={val}
			onValueChange={handleValueChange}
			showSelectedOptions={showSelectedOptions}
			filter={(option, search) => {
				const tagKey = option.tag.key.toLowerCase();
				const tagValue = option.tag.value.toLowerCase();

				if (search.includes("=")) {
					const [key, value] = search.split("=");
					return tagKey.includes(key) && tagValue.includes(value);
				}
				return tagKey.includes(search) || tagValue.includes(search);
			}}
			className="w-full min-w-[20rem]"
		/>
	);
}
