import { createContext, useContext } from "react";

const defaultValue = {
	copy: {
		createActor: "Create Actor",
		createActorUsingForm: "Create Actor using simple form",
		noActorsFound: "No Actors found",
		selectActor: (
			<>
				No Actor selected.
				<br />{" "}
				<span className="text-sm text-muted-foreground">
					Select an Actor from the list to view its details.
				</span>
			</>
		),
		goToActor: "Go to Actor",
		showActorList: "Show Actor List",
		showHiddenActors: "Show hidden Actors",
		actorId: "Actor ID",
		noActorsMatchFilter: "No Actors match the filters.",
		noMoreActors: "No more Actors to load.",

		createActorModal: {
			title: "Create Actor",
			description:
				"Choose a build to create an Actor from. Actor will be created using default settings.",
		},

		actorNotFound: "Actor not found",
		actorNotFoundDescription:
			"Change your filters to find the Actor you are looking for.",
	},
	canCreate: true,
};

export const ActorsViewContext =
	createContext<typeof defaultValue>(defaultValue);

export const useActorsView = () => {
	const context = useContext(ActorsViewContext);
	if (!context) {
		throw new Error(
			"useActorsView must be used within a ActorsViewContextProvider",
		);
	}
	return context;
};
