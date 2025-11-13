import { useState, useEffect } from "react";
import type { AppState } from "../../App";

interface TabProps {
  state: AppState;
  updateState: (updates: Partial<AppState>) => void;
  client: any;
  actorHandle: any;
}

interface ConnectionInfo {
  id: string;
  connectedAt: number;
}

export default function ConnectionsTab({ state, actorHandle }: TabProps) {
  const [currentConnectionInfo, setCurrentConnectionInfo] = useState<any>(null);
  const [allConnections, setAllConnections] = useState<ConnectionInfo[]>([]);
  const [isLoadingConnections, setIsLoadingConnections] = useState(false);

  // Update current connection info when actor handle changes
  useEffect(() => {
    if (actorHandle && state.connectionMode === "connection") {
      setCurrentConnectionInfo({
        isConnected: true,
        connectionId: actorHandle.connectionId || "N/A",
        encoding: state.encoding,
        actorName: state.actorName,
        actorKey: state.actorKey,
        actorId: state.actorId,
        lastActivity: Date.now(),
      });
    } else {
      setCurrentConnectionInfo(null);
    }
  }, [actorHandle, state]);

  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp).toLocaleString();
  };

  const handleListConnections = async () => {
    setIsLoadingConnections(true);
    try {
      const connections = await actorHandle.listConnections();
      setAllConnections(connections);
    } catch (error) {
      console.error("Failed to list connections:", error);
      alert(`Failed to list connections: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsLoadingConnections(false);
    }
  };

  if (state.actorName !== "demo") {
    return (
      <div className="section">
        <h3>Connections</h3>
        <p>Connection features are only available for the <code>demo</code> actor. Please select the demo actor in the header.</p>
      </div>
    );
  }

  return (
    <div>
      {/* Current Connection Information */}
      {state.connectionMode === "connection" && (
        <div className="section">
          <h3>Current Connection</h3>

          {!currentConnectionInfo ? (
            <div style={{
              padding: "20px",
              textAlign: "center",
              color: "var(--text-secondary)"
            }}>
              No active connection
            </div>
          ) : (
            <div style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: "10px 20px", alignItems: "center" }}>
              <strong>Status:</strong>
              <span>🟢 Connected</span>

              <strong>Connection ID:</strong>
              <code>
                {currentConnectionInfo.connectionId}
              </code>

              <strong>Encoding:</strong>
              <span>{currentConnectionInfo.encoding.toUpperCase()}</span>

              <strong>Actor Name:</strong>
              <span>{currentConnectionInfo.actorName}</span>

              <strong>Actor Key:</strong>
              <span>{currentConnectionInfo.actorKey || "(empty)"}</span>

              {currentConnectionInfo.actorId && (
                <>
                  <strong>Actor ID:</strong>
                  <code>
                    {currentConnectionInfo.actorId}
                  </code>
                </>
              )}

              <strong>Last Activity:</strong>
              <span>{formatTimestamp(currentConnectionInfo.lastActivity)}</span>
            </div>
          )}
        </div>
      )}

      {/* All Connections */}
      <div className="section">
        <h3>All Connections</h3>
        <p style={{ marginBottom: "15px" }}>
          List all active connections to this actor instance.
        </p>

        <button
          className="btn btn-primary"
          onClick={handleListConnections}
          disabled={isLoadingConnections}
        >
          {isLoadingConnections ? "Loading..." : "List All Connections"}
        </button>

        {allConnections.length > 0 && (
          <div style={{ marginTop: "15px" }}>
            <table style={{
              width: "100%",
              borderCollapse: "collapse",
              fontSize: "14px"
            }}>
              <thead>
                <tr style={{ borderBottom: "2px solid #333" }}>
                  <th style={{ padding: "8px", textAlign: "left" }}>Connection ID</th>
                  <th style={{ padding: "8px", textAlign: "left" }}>Connected At</th>
                </tr>
              </thead>
              <tbody>
                {allConnections.map((conn) => (
                  <tr key={conn.id} style={{ borderBottom: "1px solid #222" }}>
                    <td style={{ padding: "8px" }}>
                      <code>{conn.id}</code>
                    </td>
                    <td style={{ padding: "8px" }}>
                      {conn.connectedAt ? formatTimestamp(conn.connectedAt) : "N/A"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
