import { createClient } from "rivetkit/client";
import { RivetError } from "rivetkit/errors";

const client = createClient(
	process.env.RIVET_ENDPOINT ?? "http://127.0.0.1:6420",
) as any;

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function main() {
	const room = client.chatRoom.getOrCreate(`chatroom_${crypto.randomUUID()}`);

	try {
		const roomName = "Effect Lovers";
		await room.Initialize({ name: "Effect Lovers" });
		console.log(`created room ${roomName}`);

		const { memberCount } = await room.Join({ name: "Alice" });
		console.log(`Alice joined; members=${memberCount}`);

		await room.SendMessage({
			sender: "Alice",
			text: "hello from the raw client",
		});
		console.log("Alice sent a message");

		// Plain clients see declared Effect action errors as thrown RivetErrors
		// with the encoded Effect action-error metadata attached.
		try {
			await room.SendMessage({
				sender: "Mallory",
				text: "I should not be able to post",
			});
		} catch (error) {
			if (!(error instanceof RivetError)) throw error;

			if (error.code === "MemberNotInRoomError") {
				const metadata = error.metadata as {
					readonly error?: { readonly name?: string };
				};
				const memberName =
					typeof metadata.error?.name === "string"
						? metadata.error.name
						: "unknown member";
				console.warn(
					`rejected non-member message from ${memberName}: ${error.message}`,
				);
			} else {
				throw error;
			}
		}

		try {
			await room.SendMessage({
				sender: "Alice",
				text: "this contains spam",
			});
		} catch (error) {
			if (!(error instanceof RivetError)) throw error;

			if (error.code === "BannedWordsError") {
				console.warn(`rejected banned message: ${error.message}`);
			} else {
				throw error;
			}
		}

		await sleep(1_500);

		const history = await room.GetHistory();
		const transcript = history
			.map(
				(message: { sender: string; text: string }) =>
					`  ${message.sender}: ${message.text}`,
			)
			.join("\n");
		console.log(`message history:\n${transcript}`);
	} finally {
		await room.Archive();
		console.log("archived room");
	}
}

main().catch((err) => {
	console.error("raw client failed:", err);
	process.exitCode = 1;
});
