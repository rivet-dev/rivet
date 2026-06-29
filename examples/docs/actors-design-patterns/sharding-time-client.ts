import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

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
  },
});

const registry = setup({ use: { hourlyAnalytics } });
const client = createClient<typeof registry>("http://localhost:6420");

// Shard by hour: hourlyAnalytics:2024-01-15T00, hourlyAnalytics:2024-01-15T01
const shardKey = new Date().toISOString().slice(0, 13); // "2024-01-15T00"
const analytics = client.hourlyAnalytics.getOrCreate([shardKey]);
await analytics.trackEvent({ type: "page_view", url: "/home" });
