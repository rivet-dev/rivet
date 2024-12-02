// MARK: rivet.json
// {
// 	"security": [
// 		{
// 			"tags": { "name": "channel" },
// 			"allow": true
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

// MARK: channel.ts
class Channel {
	async authenticate(connection, token): Promise<boolean> {
		if (!await authenticate(token)) {
			throw new Error("unauthenticated");
		}
	}

	sendMessage(message: string) {
		this.broadcast(message);
	}
}

// MARK: Actor
const actorClient = new ActorClient();

const CHANNEL = "foo";
const USER_TOKEN = "some token from my api";
const room = await actorClient.getOrCreate({ name: "room", channel: CHANNEL }).connect(USER_TOKEN);

room.on("message", msg => {
	console.log("message");
});
room.sendMessage("hello world", msg);

