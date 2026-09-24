// "Which chat rooms are still awake, and what has Acme been saying?"
const { actors } = await rivet.actors.list({ name: "chat-room" });
const awake = actors.filter((room) => room.status === "running");

const history = await rivet.actor.action({
  actor: { name: "chat-room", key: ["acme"] },
  name: "getHistory",
});

return { awake: awake.length, history };
