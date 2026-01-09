import { useState } from "react";
import type { AppState } from "../App.tsx";

interface ConnectionScreenProps {
  onConnect: (config: AppState) => void;
}

type ActorMethod = "get" | "getOrCreate" | "getForId" | "create";

export default function ConnectionScreen({ onConnect }: ConnectionScreenProps) {
  const [actorMethod, setActorMethod] = useState<ActorMethod>("getOrCreate");
  const [actorName, setActorName] = useState("demo");
  const [actorKey, setActorKey] = useState("");
  const [actorId, setActorId] = useState("");
  const [actorRegion, setActorRegion] = useState("");
  const [createInput, setCreateInput] = useState("");
  const [encoding, setEncoding] = useState<"json" | "cbor" | "bare">("bare");
  const [connectionMode, setConnectionMode] = useState<"connection" | "handle">("handle");
  const [isConnecting, setIsConnecting] = useState(false);

  const handleConnect = async () => {
    setIsConnecting(true);

    const config: AppState = {
      actorMethod,
      actorName,
      actorKey,
      actorId,
      actorRegion,
      createInput,
      encoding,
      connectionMode,
      isConnected: true,
    };

    onConnect(config);
    setIsConnecting(false);
  };

  return (
    <div className="connection-screen">
      <div className="connection-modal">
        <div className="modal-header">
          <h1>Connect to Actor</h1>
          <p>Configure your RivetKit connection</p>
        </div>

        <div className="modal-content">
          <div className="form-section">
            <h3>Actor Configuration</h3>
            <div className="form-group">
              <label>Method</label>
              <div className="toggle-group method-toggle">
                <button
                  className={`toggle-button ${actorMethod === "get" ? "active" : ""}`}
                  onClick={() => setActorMethod("get")}
                >
                  Get
                </button>
                <button
                  className={`toggle-button ${actorMethod === "getOrCreate" ? "active" : ""}`}
                  onClick={() => setActorMethod("getOrCreate")}
                >
                  Get or Create
                </button>
                <button
                  className={`toggle-button ${actorMethod === "getForId" ? "active" : ""}`}
                  onClick={() => setActorMethod("getForId")}
                >
                  Get for ID
                </button>
                <button
                  className={`toggle-button ${actorMethod === "create" ? "active" : ""}`}
                  onClick={() => setActorMethod("create")}
                >
                  Create
                </button>
              </div>
            </div>

            <div className="form-group">
              <label>Actor Name</label>
              <select
                className="form-control"
                value={actorName}
                onChange={(e) => setActorName(e.target.value)}
              >
                <option value="demo">Demo Actor</option>
              </select>
            </div>

            {actorMethod === "getForId" ? (
              <div className="form-group">
                <label>Actor ID</label>
                <input
                  className="form-control"
                  type="text"
                  value={actorId}
                  onChange={(e) => setActorId(e.target.value)}
                  placeholder="Actor ID"
                  required
                />
              </div>
            ) : (
              <div className="form-group">
                <label>Key</label>
                <input
                  className="form-control"
                  type="text"
                  value={actorKey}
                  onChange={(e) => setActorKey(e.target.value)}
                  placeholder="Optional key for actor instance"
                />
              </div>
            )}

            {(actorMethod === "create" || actorMethod === "getOrCreate") && (
              <>
                <div className="form-group">
                  <label>Region</label>
                  <input
                    className="form-control"
                    type="text"
                    value={actorRegion}
                    onChange={(e) => setActorRegion(e.target.value)}
                    placeholder="Optional region"
                  />
                </div>
                <div className="form-group">
                  <label>Input Data</label>
                  <textarea
                    className="form-control textarea"
                    value={createInput}
                    onChange={(e) => setCreateInput(e.target.value)}
                    placeholder="Optional JSON input data"
                    rows={3}
                  />
                </div>
              </>
            )}
          </div>

          <div className="form-section">
            <h3>Connection Settings</h3>
            <div className="form-group">
              <label>Mode</label>
              <div className="toggle-group">
                <button
                  className={`toggle-button ${connectionMode === "handle" ? "active" : ""}`}
                  onClick={() => setConnectionMode("handle")}
                >
                  Handle
                </button>
                <button
                  className={`toggle-button ${connectionMode === "connection" ? "active" : ""}`}
                  onClick={() => setConnectionMode("connection")}
                >
                  Connection
                </button>
              </div>
            </div>

            <div className="form-group">
              <label>Encoding</label>
              <div className="toggle-group">
                <button
                  className={`toggle-button ${encoding === "bare" ? "active" : ""}`}
                  onClick={() => setEncoding("bare")}
                >
                  Bare
                </button>
                <button
                  className={`toggle-button ${encoding === "cbor" ? "active" : ""}`}
                  onClick={() => setEncoding("cbor")}
                >
                  CBOR
                </button>
                <button
                  className={`toggle-button ${encoding === "json" ? "active" : ""}`}
                  onClick={() => setEncoding("json")}
                >
                  JSON
                </button>
              </div>
            </div>
          </div>


          <div className="modal-actions">
            <button
              className="btn btn-primary connect-button"
              onClick={handleConnect}
              disabled={isConnecting}
              aria-busy={isConnecting ? "true" : "false"}
            >
              {isConnecting ? "Connecting..." : "Connect to Actor"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
