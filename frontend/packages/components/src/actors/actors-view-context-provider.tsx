import { createContext, useContext } from "react";

export const ActorsViewContext = createContext<{
	copy: {
		createActor: string;
		createActorUsingForm: string;
		noActorsFound: string;
		selectActor: string;
		goToActor: string;
		showActorList: string;
		actorId: string;
	};
	requiresManager: boolean;
}>({
	copy: {
		createActor: "Create Actor",
		createActorUsingForm: "Create Actor using simple form",
		noActorsFound: "No actors found",
		selectActor: "Please select an Actor from the list.",
		goToActor: "Go to Actor",
		showActorList: "Show Actor List",
		actorId: "Actor ID",
	},
	requiresManager: true,
});

export const useActorsView = () => {
	const context = useContext(ActorsViewContext);
	if (!context) {
		throw new Error(
			"useActorsView must be used within a ActorsViewContextProvider",
		);
	}
	return context;
};
