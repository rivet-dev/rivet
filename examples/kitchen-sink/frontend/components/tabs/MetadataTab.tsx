import { useState } from "react";
import type { AppState } from "../../App.tsx";

interface TabProps {
  state: AppState;
  updateState: (updates: Partial<AppState>) => void;
  client: any;
  actorHandle: any;
}

export default function MetadataTab({ state, actorHandle }: TabProps) {
  const [metadata, setMetadata] = useState<any>(null);
  const [actorState, setActorState] = useState<any>(null);
  const [connState, setConnState] = useState<any>(null);
  const [loadingStates, setLoadingStates] = useState({
    metadata: false,
    actorState: false,
    connState: false
  });

  const callAction = async (actionName: string, args: any[] = []) => {
    return await actorHandle[actionName](...args);
  };

  const handleGetMetadata = async () => {
    setLoadingStates(prev => ({ ...prev, metadata: true }));
    setMetadata(null);

    try {
      const result = await callAction("getMetadata");
      setMetadata(result);
    } catch (error) {
      setMetadata({
        error: error instanceof Error ? error.message : String(error)
      });
    } finally {
      setLoadingStates(prev => ({ ...prev, metadata: false }));
    }
  };

  const handleGetActorState = async () => {
    setLoadingStates(prev => ({ ...prev, actorState: true }));
    setActorState(null);

    try {
      const result = await callAction("getActorState");
      setActorState(result);
    } catch (error) {
      setActorState({
        error: error instanceof Error ? error.message : String(error)
      });
    } finally {
      setLoadingStates(prev => ({ ...prev, actorState: false }));
    }
  };

  const handleGetConnState = async () => {
    setLoadingStates(prev => ({ ...prev, connState: true }));
    setConnState(null);

    try {
      const result = await callAction("getConnState");
      setConnState(result);
    } catch (error) {
      setConnState({
        error: error instanceof Error ? error.message : String(error)
      });
    } finally {
      setLoadingStates(prev => ({ ...prev, connState: false }));
    }
  };

  if (state.actorName !== "demo") {
    return (
      <div className="section">
        <h3>Metadata</h3>
        <p>Metadata features are only available for the <code>demo</code> actor. Please select the demo actor in the header.</p>
      </div>
    );
  }

  return (
    <div>
      <div className="section">
        <h3>Actor Metadata</h3>
        <p style={{ marginBottom: "15px" }}>
          Actors can access metadata about themselves including their name, tags, and region.
          This information is useful for logging, monitoring, and conditional behavior.
        </p>

        <button
          className="btn btn-primary"
          onClick={handleGetMetadata}
          disabled={loadingStates.metadata}
        >
          {loadingStates.metadata ? "Loading..." : "Get Metadata"}
        </button>

        {metadata && (
          <div className="response">
            <pre style={{ margin: 0, fontSize: "13px", overflow: "auto" }}>
              {JSON.stringify(metadata, null, 2)}
            </pre>
          </div>
        )}
      </div>

      <div className="section">
        <h3>Actor State</h3>
        <p style={{ marginBottom: "15px" }}>
          Current state of the actor. This state is shared across all connections to the actor.
        </p>

        <button
          className="btn btn-primary"
          onClick={handleGetActorState}
          disabled={loadingStates.actorState}
        >
          {loadingStates.actorState ? "Loading..." : "Get Actor State"}
        </button>

        {actorState && (
          <div className="response">
            <pre style={{ margin: 0, fontSize: "13px", overflow: "auto" }}>
              {JSON.stringify(actorState, null, 2)}
            </pre>
          </div>
        )}
      </div>

      <div className="section">
        <h3>Connection State</h3>
        <p style={{ marginBottom: "15px" }}>
          State specific to the current connection. Each connection has its own isolated state.
        </p>

        <button
          className="btn btn-primary"
          onClick={handleGetConnState}
          disabled={loadingStates.connState}
        >
          {loadingStates.connState ? "Loading..." : "Get Connection State"}
        </button>

        {connState && (
          <div className="response">
            <pre style={{ margin: 0, fontSize: "13px", overflow: "auto" }}>
              {JSON.stringify(connState, null, 2)}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}