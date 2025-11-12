import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { createContext, useContext } from "react";
import { useDataProvider } from "@/components/actors";

export const InspectorContext = createContext({
	isInspectorAvailable: false,
	connect: (_: { url: string }) => {},
	disconnect: () => {},
});

export const useInspectorContext = () => {
	return useContext(InspectorContext);
};

export const InspectorContextProvider = ({
	children,
}: {
	children: React.ReactNode;
}) => {
	const navigate = useNavigate();
	const status = useInspectorStatus();

	const connect = ({ url }: { url: string }) => {
		return navigate({
			to: ".",
			search: (s) => ({
				...s,
				u: url,
			}),
		});
	};

	const disconnect = () => {
		return navigate({
			to: ".",
			search: {},
		});
	};

	return (
		<InspectorContext.Provider
			value={{
				isInspectorAvailable:
					status === "connected" || status === "reconnecting",
				connect,
				disconnect,
			}}
		>
			{children}
		</InspectorContext.Provider>
	);
};

export const useInspectorEndpoint = () => {
	return useSearch({ from: "/_context", select: (s) => s.u });
};

export const useInspectorStatus = () => {
	const url = useInspectorEndpoint();
	const dataProvider = useDataProvider();
	const enabled = !!url;
	const { data, isSuccess, isRefetchError, isLoading } = useQuery({
		...dataProvider.statusQueryOptions(),
		enabled: !!url,
		refetchInterval: 1_000,
		placeholderData: keepPreviousData,
		retry: 0,
	});

	if (data && enabled && isRefetchError) {
		return "reconnecting";
	}

	if ((isLoading || isRefetchError) && enabled) {
		return "connecting";
	}

	if (isSuccess && enabled) {
		return "connected";
	}

	return "disconnected";
};
