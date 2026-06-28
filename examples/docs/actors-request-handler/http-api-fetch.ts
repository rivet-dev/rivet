// docs:start fetch
// Replace with your actor ID and token
const actorId = "your-actor-id";
const token = "your-token";

const response = await fetch(
  `https://api.rivet.dev/gateway/${actorId}/request/increment`,
  {
    method: "POST",
    headers: {
      "x-rivet-token": token,
    },
  }
);
const data = await response.json();
console.log(data); // { count: 1 }
// docs:end fetch
export {};
