import { ActorsList } from "./actors-list";

interface ActorsListPanelProps {
	projectNameId: string;
	environmentNameId: string;
	actorId: string | undefined;
	tags: Record<string, string>;
	showDestroyed: boolean;
}

export function ActorsListPanel({
	actorId,
	projectNameId,
	environmentNameId,
	tags,
	showDestroyed,
}: ActorsListPanelProps) {
	return (
		<>
			<ActorsList
				projectNameId={projectNameId}
				environmentNameId={environmentNameId}
				actorId={actorId}
				tags={tags}
				showDestroyed={showDestroyed}
			/>
		</>
	);
}
