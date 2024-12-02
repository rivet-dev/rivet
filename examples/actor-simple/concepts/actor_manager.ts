// Problem:
// We need to be able to create & connect to actors from the client without adding complexity.
//
// Option A: On-demand roomts
//
// - PartyKit have a sort of rooms that can be automatically created
// - These rely on authentication first
// - Unsure if there's some sort of "room middleware" or something
//
// Option B: Proxy worker
// - Use an actor to proxy requests to the other actor
// - This is how Cloudflare Workers & Erlang works
// - This is the purest actor model
// - Docs
//	- PartyKit on demand: https://docs.partykit.io/how-partykit-works/#on-demand
//
// Option C: Manager actor
// - Use an actor to manage actors and return new addresses
// - This is the Erlang way and also the Socket.io way
// - Docs
//	- Join room: https://socket.io/docs/v4/server-api/#socketjoinroom (sort of)
//
// Option A is better because it's likely simpler to understand.
//
// Use case example:
// - Chat room
// - Counter for arbitrary values
//
// Is there a way to build a room manager actor, then return a URL for authenticated access to an actor?
//
// Upsides:
// - Remove use of get-or-create
//
// Downsides:
// - More complicated
// - Creates a single point of failure & choke point
//
// Things to figure out:
// - How do you scale the room manager?
// - Can we re-implement this as client-first?
//
// Similar examples:
// - User management (who creates the user actor?)
//	- This would require an actor created on a request or something like that
//	- Look at Cowboy for this?
//
// Unknowns:
// - How do we handle auth to the new actor?

// MARK: rivet.json
// {
//  // This calls get or create on deploy for a durable actor. This address can be used to connect to the manager.
//  // HOWEVER, this won't support scaling and won't be production ready
// 	"supervisors": [
// 		{
// 			"tags": { "name": "manager" },
// 		}
// 	],
// 	"builds": [
// 		{
// 			"tags": { "name": "manager" },
// 			"script": "manager.ts"
// 		},
// 		{
// 			"tags": { "name": "channel" },
// 			"script": "channel.ts"
// 		},
// 	]
// }

// MARK: manager.ts
class ChannelManager {
	join(channel: string): Actor {
		return await this.actors.getOrCreate({ name: "channel", channel }).handle();
	}
}

// MARK: channel.ts
class Channel {
	sendMessage(message: string) {
		this.broadcast(message);
	}
}

// MARK: client.ts
const actorClient = new ActorClient();

const manager = actorClient.connect("https://xxxx.rivet.run");
const room = await manager.join("foo");

room.on("message", msg => {
	console.log("message");
});
room.sendMessage("hello world", msg);


