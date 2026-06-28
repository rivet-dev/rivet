import { actor, UserError } from "rivetkit";

const user = actor({
  state: { username: "" },
  actions: {
    updateUsername: (c, username: string) => {
      // Validate username
      if (username.length > 32) {
        // Throw a simple error with a message
        throw new UserError("Username is too long", {
          code: "username_too_long",
          metadata: {
            maxLength: 32
          }
        });
      }

      // Update username
      c.state.username = username;
    }
  }
});
