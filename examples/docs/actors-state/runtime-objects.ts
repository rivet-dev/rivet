import { actor } from "rivetkit";

const room = actor({
  state: { messages: [] as string[] },

  // EventTarget can't be serialized, so it lives in vars
  createVars: () => ({ events: new EventTarget() }),

  actions: {
    send: (c, text: string) => {
      c.state.messages.push(text);
      c.vars.events.dispatchEvent(new CustomEvent("message", { detail: text }));
    }
  }
});
