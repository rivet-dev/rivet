import { IdentifyUser } from "@/components/third-party-providers";
import {
	selfProfileQueryOptions,
	useIdentityTokenMutation,
	useLogoutMutation,
} from "@/domains/user/queries";
import type { Rivet } from "@rivet-gg/api-full";
import { useSuspenseQuery } from "@tanstack/react-query";
import { createContext, useContext } from "react";
import { bootstrapQueryOptions } from "../queries/bootstrap";

export interface AuthContext {
	profile: Rivet.identity.GetProfileResponse | undefined;
	isProfileLoading: boolean;
	logout: () => void;
	refreshToken: () => Promise<unknown>;
}

const AuthContext = createContext<AuthContext | null>(null);

export function AuthProvider({ children }: { children: React.ReactNode }) {
	const { mutateAsync } = useIdentityTokenMutation();
	const {
		data: profile,
		isRefetching,
		isFetching,
	} = useSuspenseQuery(selfProfileQueryOptions());

	const { mutate: logout } = useLogoutMutation();

	useSuspenseQuery(bootstrapQueryOptions());

	return (
		<AuthContext.Provider
			value={{
				profile,
				logout,
				isProfileLoading: isRefetching || isFetching,
				refreshToken: mutateAsync,
			}}
		>
			<IdentifyUser />
			{children}
		</AuthContext.Provider>
	);
}

export function useAuth() {
	const context = useContext(AuthContext);
	if (!context) {
		throw new Error("useAuth must be used within an AuthProvider");
	}
	return context;
}
