import { type UseNavigateResult, useNavigate } from "@tanstack/react-router";
import {
	type ReactNode,
	createContext,
	startTransition,
	useCallback,
	useContext,
} from "react";

type GroupCommandPanelPage = {
	key: "group";
	params: { groupId: string };
};

type ProjectCommandPanelPage = {
	key: "project";
	params: { projectNameId: string };
};

type EnvironmentCommandPanelPage = {
	key: "environment";
	params: { projectNameId: string; environmentNameId: string };
};

type BackendCommandPanelPage = {
	key: "backend";
	params: { projectNameId: string };
};

type BackendEnvironmentPanelPage = {
	key: "environment";
	params: { projectNameId: string; environmentNameId: string };
};

type CommandPanelPages =
	| GroupCommandPanelPage
	| ProjectCommandPanelPage
	| EnvironmentCommandPanelPage
	| BackendCommandPanelPage
	| BackendEnvironmentPanelPage
	| never;

export type CommandPanelPage = CommandPanelPages;

const CommandPanelNavigationContext = createContext<{
	changePage: (page: CommandPanelPage) => void;
	close: () => void;
	navigate: UseNavigateResult<string>;
}>({
	changePage: () => {},
	close: () => {},
	navigate: async () => {},
});

export function CommandPanelNavigationProvider({
	children,
	onClose,
	onChangePage,
	isLoading,
}: {
	children: ReactNode;
	onClose: () => void;
	onChangePage: (page: CommandPanelPage) => void;
	isLoading: boolean;
}) {
	const routerNavigate = useNavigate();

	const navigate: UseNavigateResult<string> = useCallback(
		async (params) => {
			startTransition(() => {
				onClose();
				routerNavigate(params);
			});
		},
		[onClose, routerNavigate],
	);

	const handleChangePage = (page: CommandPanelPage) => {
		if (isLoading) {
			return;
		}
		onChangePage(page);
	};

	return (
		<CommandPanelNavigationContext.Provider
			value={{ changePage: handleChangePage, close: onClose, navigate }}
		>
			{children}
		</CommandPanelNavigationContext.Provider>
	);
}

export const useCommandPanelNavigation = () =>
	useContext(CommandPanelNavigationContext);
