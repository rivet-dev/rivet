import { setup } from "rivetkit";
import { userApp } from "./user-app";
import { userAppList } from "./user-app-list";

export const registry = setup({
	use: { userApp, userAppList },
});
