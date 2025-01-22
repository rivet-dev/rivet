import {
	type Dispatch,
	type ReactNode,
	type SetStateAction,
	createContext,
	useContext,
} from "react";
import { useLocalStorage } from "usehooks-ts";

interface Settings {
	showTimestmaps: boolean;
	autoFollowLogs: boolean;
}

export const ActorDetailsSettingsContext = createContext<
	[Settings, Dispatch<SetStateAction<Settings>>, unknown]
>([{ showTimestmaps: false, autoFollowLogs: true }, () => {}, {}]);

export const useActorDetailsSettings = () => {
	const value = useContext(ActorDetailsSettingsContext);
	return value;
};

interface ActorDetailsSettingsProviderProps {
	children: ReactNode;
}

export const ActorDetailsSettingsProvider = ({
	children,
}: ActorDetailsSettingsProviderProps) => {
	const localStorage = useLocalStorage<Settings>("actor-details-settings", {
		showTimestmaps: false,
		autoFollowLogs: true,
	});

	return (
		<ActorDetailsSettingsContext.Provider value={localStorage}>
			{children}
		</ActorDetailsSettingsContext.Provider>
	);
};
