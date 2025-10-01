import { Clerk } from "@clerk/clerk-js";
import { cloudEnv } from "./env";

export const clerk = new Clerk(cloudEnv().VITE_APP_CLERK_PUBLISHABLE_KEY);
