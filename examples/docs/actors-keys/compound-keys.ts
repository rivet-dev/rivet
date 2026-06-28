import { actor, setup } from "rivetkit";
import { createClient } from "rivetkit/client";

const chatRoom = actor({ state: { messages: [] as string[] }, actions: {} });
const gameRoom = actor({ state: { players: [] as string[] }, actions: {} });
const workspace = actor({ state: { data: {} }, actions: {} });

const registry = setup({ use: { chatRoom, gameRoom, workspace } });
const client = createClient<typeof registry>("http://localhost:6420");

// Example user data
const userId = "user-123";
const gameId = "game-456";
const tenantId = "tenant-789";
const workspaceId = "workspace-abc";

// User-specific chat rooms
const userRoomHandle = client.chatRoom.getOrCreate(["user", userId, "private"]);

// Game rooms by region and difficulty
const gameRoomHandle = client.gameRoom.getOrCreate(["us-west", "hard", gameId]);

// Multi-tenant resources
const workspaceHandle = client.workspace.getOrCreate(["tenant", tenantId, workspaceId]);
