import { useAtomValue } from "jotai";
import { useMemo } from "react";
import { Combobox } from "@/components";
import { actorTagsAtom } from "./actor-context";
import { ActorTag } from "./actor-tags";

interface ActorTagsSelectProps {
	value: Record<string, string>;
	onValueChange: (value: Record<string, string>) => void;
	showSelectedOptions?: number;
}

export function ActorTagsSelect({
	value,
	onValueChange,
	showSelectedOptions,
}: ActorTagsSelectProps) {
	const data = useAtomValue(actorTagsAtom);

	const valArray = useMemo(() => Object.entries(value), [value]);
	const tags = useMemo(() => {
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
		return tags;
	}, [valArray, data]);

	const val = useMemo(
		() =>
			valArray.map(([key, value]) => {
				return [key, value].join("=");
			}),
		[valArray],
	);

	const options = useMemo(
		() =>
			tags.map((tag) => {
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
			}),
		[tags],
	);

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
