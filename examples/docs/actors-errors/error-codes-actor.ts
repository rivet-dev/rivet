import { actor, UserError } from "rivetkit";

const user = actor({
  state: { username: "" },
  actions: {
    updateUsername: (c, username: string) => {
      if (username.length < 3) {
        throw new UserError("Username is too short", {
          code: "username_too_short"
        });
      }

      if (username.length > 32) {
        throw new UserError("Username is too long", {
          code: "username_too_long"
        });
      }

      // Update username
      c.state.username = username;
    }
  }
});
