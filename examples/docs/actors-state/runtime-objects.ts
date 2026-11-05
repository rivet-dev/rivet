import { actor } from "rivetkit";

const room = actor({
  // EventTarget can't be serialized, so it lives in vars
  createVars: () => ({
    messages: [] as string[],
    events: new EventTarget(),
  }),

  actions: {
    send: (c, text: string) => {
      c.vars.messages.push(text);
      c.vars.events.dispatchEvent(new CustomEvent("message", { detail: text }));
    }
  }
});
