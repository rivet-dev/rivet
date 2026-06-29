import { actor } from "rivetkit";

interface ChatRoomInput {
  roomName: string;
  isPrivate: boolean;
  maxUsers?: number;
}

interface ChatRoomState {
  name: string;
  isPrivate: boolean;
  maxUsers: number;
  users: Record<string, boolean>;
  messages: string[];
}

// Mock function for demonstration
function setupPrivateRoomLogging(roomName: string) {
  console.log(`Setting up logging for private room: ${roomName}`);
}

const chatRoom = actor({
  createState: (c, input: ChatRoomInput): ChatRoomState => ({
    name: input?.roomName ?? "Unnamed Room",
    isPrivate: input?.isPrivate ?? false,
    maxUsers: input?.maxUsers ?? 50,
    users: {},
    messages: [],
  }),

  onCreate: (c, input: ChatRoomInput) => {
    console.log(`Creating room: ${input.roomName}`);

    // Setup external services based on input
    if (input.isPrivate) {
      setupPrivateRoomLogging(input.roomName);
    }
  },

  actions: {
    // Input remains accessible in actions via initial state
    getRoomInfo: (c) => ({
      name: c.state.name,
      isPrivate: c.state.isPrivate,
      maxUsers: c.state.maxUsers,
      currentUsers: Object.keys(c.state.users).length,
    }),
  },
});
