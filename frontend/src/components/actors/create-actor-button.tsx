import { faPlus, Icon } from "@rivet-gg/icons";
import { useInfiniteQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { Button, type ButtonProps, WithTooltip } from "@/components";
import { useActorsView } from "./actors-view-context-provider";
import { useDataProvider } from "./data-provider";

export function CreateActorButton({
	label,
	iconOnly,
	...props
}: ButtonProps & { label?: string; iconOnly?: boolean }) {
	const navigate = useNavigate();

	const provider = useDataProvider();

	const { data } = useInfiniteQuery(provider.buildsQueryOptions());

	const { copy } = useActorsView();

	const canCreate = data && data.length > 0;

	if (!provider.features.canCreateActors) {
		return null;
	}

	const onClick = () => {
		navigate({
			to: ".",
			search: (prev) => ({
				...prev,
				modal: "create-actor",
			}),
		});
	};

	const content = iconOnly ? (
		<div>
			<Button
				disabled={!canCreate}
				size="icon-sm"
				variant="ghost"
				onClick={onClick}
				aria-label={label ?? copy.createActor}
				{...props}
			>
				<Icon icon={faPlus} />
			</Button>
		</div>
	) : (
		<div>
			<Button
				disabled={!canCreate}
				size="sm"
				variant="ghost"
				onClick={onClick}
				startIcon={<Icon icon={faPlus} />}
				{...props}
			>
				{label ?? copy.createActor}
			</Button>
		</div>
	);

	if (canCreate) {
		return content;
	}

	return (
		<WithTooltip
			trigger={content}
			content={
				data && data.length <= 0
					? "Please deploy a build first."
					: copy.createActorUsingForm
			}
		/>
	);
}
