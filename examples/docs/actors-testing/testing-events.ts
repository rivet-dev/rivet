import { test, expect, vi } from "vitest";
import { setupTest } from "rivetkit/test";
import { actor, setup } from "rivetkit";

interface ChatMessage {
  username: string;
  message: string;
}

// Define the chat room actor
const chatRoom = actor({
  state: {
    messages: [] as ChatMessage[]
  },
  actions: {
    sendMessage: (c, username: string, message: string) => {
      c.state.messages.push({ username, message });
      c.broadcast("newMessage", username, message);
    },
    getHistory: (c) => {
      return c.state.messages;
    },
  },
});

// Create the registry
const registry = setup({
  use: { chatRoom }
});

// Test event emission
test("actor should emit events", async (testCtx) => {
  const { client } = await setupTest(testCtx, registry);
  const chatRoomHandle = client.chatRoom.getOrCreate(["test"]);

  // Set up event handler with a mock function
  const mockHandler = vi.fn();
  const conn = chatRoomHandle.connect();
  conn.on("newMessage", mockHandler);

  // Trigger the event
  await conn.sendMessage("testUser", "Hello world");

  // Wait for the event to be emitted
  await vi.waitFor(() => {
    expect(mockHandler).toHaveBeenCalledWith("testUser", "Hello world");
  });
});
