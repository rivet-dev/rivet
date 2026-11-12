import { actor } from "rivetkit";

const chatRoom = actor({
  // Build a non-serializable emitter on each start
  createVars: () => ({
    messages: [] as string[],
    emitter: createEventEmitter(),
  }),

  actions: {
    broadcast: (c, text: string) => {
      c.vars.messages.push(text);
      // Use the ephemeral emitter
      c.vars.emitter.emit("message", text);
    }
  }
});

// Mock event emitter for demonstration
interface EventEmitter {
  on: (event: string, callback: (data: unknown) => void) => void;
  emit: (event: string, data: unknown) => void;
}

function createEventEmitter(): EventEmitter {
  const listeners: Record<string, ((data: unknown) => void)[]> = {};
  return {
    on: (event, callback) => {
      listeners[event] = listeners[event] || [];
      listeners[event].push(callback);
    },
    emit: (event, data) => {
      listeners[event]?.forEach(cb => cb(data));
    }
  };
}
