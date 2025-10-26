import {
	createContext,
	type Dispatch,
	type ReactNode,
	type SetStateAction,
	useContext,
} from "react";
import { useLocalStorage } from "usehooks-ts";

export interface Settings {
	showTimestamps: boolean;
	autoFollowLogs: boolean;
}

export const ActorDetailsSettingsContext = createContext<
	[Settings, Dispatch<SetStateAction<Settings>>, unknown]
>([{ showTimestamps: false, autoFollowLogs: true }, () => {}, {}]);

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
		showTimestamps: false,
		autoFollowLogs: true,
	});

	return (
		<ActorDetailsSettingsContext.Provider value={localStorage}>
			{children}
		</ActorDetailsSettingsContext.Provider>
	);
};
