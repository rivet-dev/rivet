import { createClient } from "@rivetkit/react";
import { useState, useMemo, useEffect } from "react";
import type { Registry } from "../backend/registry";
import ConnectionScreen from "./components/ConnectionScreen";
import InteractionScreen from "./components/InteractionScreen";

export interface AppState {
  // Configuration
  encoding: "json" | "cbor" | "bare";
  connectionMode: "handle" | "connection";

  // Actor management
  actorMethod: "get" | "getOrCreate" | "getForId" | "create";
  actorName: string;
  actorKey: string;
  actorId: string;
  actorRegion: string;
  createInput: string;

  // Connection state
  isConnected: boolean;
  connectionError?: string;
}

function App() {
  const [state, setState] = useState<AppState | null>(null);

  const handleConnect = (config: AppState) => {
    setState(config);
  };

  const handleDisconnect = () => {
    setState(null);
  };

  const updateState = (updates: Partial<AppState>) => {
    setState(prev => prev ? { ...prev, ...updates } : null);
  };

  // Create client with user-selected encoding
  const client = useMemo(() => {
    if (!state) return null;

    return createClient<Registry>({
      endpoint: "http://localhost:6420",
      encoding: state.encoding,
    });
  }, [state?.encoding]);

  // Create the connection/handle once based on state
  const [actorHandle, setActorHandle] = useState<any>(null);

  useEffect(() => {
    if (!state || !client) {
      setActorHandle(null);
      return;
    }

    const accessor = (client as any)[state.actorName];
    const key = state.actorKey ? [state.actorKey] : [];

    const initHandle = async () => {
      let baseHandle: any;
      switch (state.actorMethod) {
        case "get":
          baseHandle = accessor.get(key);
          break;
        case "getOrCreate": {
          const createInput = state.createInput ? JSON.parse(state.createInput) : undefined;
          baseHandle = accessor.getOrCreate(key, { createWithInput: createInput });
          break;
        }
        case "getForId":
          if (!state.actorId) {
            throw new Error("Actor ID is required for getForId method");
          }
          baseHandle = accessor.getForId(state.actorId);
          break;
        case "create": {
          const createInput = state.createInput ? JSON.parse(state.createInput) : undefined;
          baseHandle = await accessor.create(key, { input: createInput });
          break;
        }
        default:
          throw new Error(`Unknown actor method: ${state.actorMethod}`);
      }

      // Apply connection mode
      const handle = state.connectionMode === "connection"
        ? baseHandle.connect()
        : baseHandle;

      setActorHandle(handle);
    };

    initHandle();
  }, [state, client]);

  return (
    <div className="app">
      <ConnectionScreen onConnect={handleConnect} />
      {state && actorHandle && (
        <InteractionScreen
          state={state}
          updateState={updateState}
          client={client}
          actorHandle={actorHandle}
          onDisconnect={handleDisconnect}
        />
      )}
    </div>
  );
}

export default App;
