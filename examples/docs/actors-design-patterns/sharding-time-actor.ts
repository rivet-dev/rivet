import { actor, setup } from "rivetkit";

interface Event {
  type: string;
  url: string;
}

const hourlyAnalytics = actor({
  state: { events: [] as Event[] },
  actions: {
    trackEvent: (c, event: Event) => {
      c.state.events.push(event);
    },
    getEvents: (c) => c.state.events,
  },
});

export const registry = setup({
  use: { hourlyAnalytics },
});
