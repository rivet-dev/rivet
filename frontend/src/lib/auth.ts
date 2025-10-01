import { Clerk } from "@clerk/clerk-js";
import { cloudEnv } from "./env";

export const clerk =
	__APP_TYPE__ === "cloud"
		? new Clerk(cloudEnv().VITE_APP_CLERK_PUBLISHABLE_KEY)
		: (null as unknown as Clerk);
