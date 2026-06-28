import { actor } from "rivetkit";

interface UserInput {
  email: string;
  name: string;
}

const userActor = actor({
  createState: (c, input: UserInput) => ({
    email: input.email,
    name: input.name,
  }),
  actions: {
    deleteAccount: (c) => {
      c.destroy();
    },
  },
});
