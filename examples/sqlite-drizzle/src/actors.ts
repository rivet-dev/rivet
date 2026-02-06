// import { actor, setup } from "rivetkit";
// import { db } from "@rivetkit/db/drizzle";
// import * as schema from "./db/schema.ts";
// import migrations from "../drizzle/migrations.js";

// export const counter = actor({
// 	db: db({ schema, migrations }),
// 	state: {
// 		count: 0,
// 	},
// 	actions: {
// 		increment: (c, x: number) => {
// 			// createState or state fix fix fix
// 			c.db.c.state.count += x;
// 			return c.state.count;
// 		},
// 	},
// });

// export const registry = setup({
// 	use: { counter },
// });
