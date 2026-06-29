import { actor, UserError } from "rivetkit";

const user = actor({
state: { username: "" },
actions: {
updateUsername: (c, username: string) => {
if (username.length < 3) {
throw new UserError("Username too short", {
code: "username_too_short",
metadata: { minLength: 3, actual: username.length },
});
}
c.state.username = username;
},
},
});
