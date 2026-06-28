import { actor } from "rivetkit";

const analyticsActor = actor({
  state: { events: [] as string[] },

  actions: {
    track: (c, event: string) => {
      c.state.events.push(event);

      // The actor will wait for this to complete before sleeping.
      c.waitUntil(
        fetch("https://analytics.example.com/ingest", {
          method: "POST",
          body: JSON.stringify({ event }),
        }).then(() => {})
      );
    },
  },
});
