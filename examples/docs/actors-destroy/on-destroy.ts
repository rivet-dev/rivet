import { actor } from "rivetkit";

interface UserState {
  email: string;
  name: string;
}

// Example email service interface
const emailService = {
  send: async (options: { from: string; to: string; subject: string; text: string }) => {},
};

const userActor = actor({
  state: { email: "", name: "" } as UserState,
  onDestroy: async (c) => {
    await emailService.send({
      from: "noreply@example.com",
      to: c.state.email,
      subject: "Account Deleted",
      text: `Goodbye ${c.state.name}, your account has been deleted.`,
    });
  },
  actions: {
    deleteAccount: (c) => {
      c.destroy();
    },
  },
});
