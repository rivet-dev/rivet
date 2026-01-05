import { useState, useEffect, useRef } from "react";
import type { AppState } from "../../App";

interface TabProps {
  state: AppState;
  updateState: (updates: Partial<AppState>) => void;
  client: any;
  actorHandle: any;
}

interface Message {
  id: string;
  type: "sent" | "received";
  data: string;
  timestamp: number;
  isBinary: boolean;
}

export default function RawWebSocketTab({ state }: TabProps) {
  const [message, setMessage] = useState('{"type": "ping", "timestamp": 0}');
  const [isBinary, setIsBinary] = useState(false);
  const [messages, setMessages] = useState<Message[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [connectionStatus, setConnectionStatus] = useState("disconnected");
  const [connectionError, setConnectionError] = useState("");

  const websocketRef = useRef<WebSocket | null>(null);
  const messageIdCounter = useRef(0);

  const getWebSocketUrl = () => {
    const wsProtocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const actorPath = state.actorKey ?
      `/actors/${state.actorName}/${encodeURIComponent(state.actorKey)}/ws` :
      `/actors/${state.actorName}/ws`;
    return `${wsProtocol}//localhost:8080${actorPath}`;
  };

  const addMessage = (type: "sent" | "received", data: string, isBinary = false) => {
    const message: Message = {
      id: `msg-${++messageIdCounter.current}`,
      type,
      data,
      timestamp: Date.now(),
      isBinary,
    };
    setMessages(prev => [...prev, message].slice(-50)); // Keep last 50 messages
  };

  const connectWebSocket = () => {
    if (websocketRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    setConnectionStatus("connecting");
    setConnectionError("");

    try {
      const ws = new WebSocket(getWebSocketUrl());
      websocketRef.current = ws;

      ws.onopen = () => {
        setIsConnected(true);
        setConnectionStatus("connected");
        addMessage("received", "WebSocket connected", false);
      };

      ws.onmessage = (event) => {
        const data = event.data;
        const isBinary = data instanceof ArrayBuffer || data instanceof Blob;

        if (isBinary) {
          // Convert binary data to hex string for display
          if (data instanceof Blob) {
            data.arrayBuffer().then(ab => {
              const bytes = new Uint8Array(ab);
              const hex = Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join(' ');
              addMessage("received", `[Binary: ${bytes.length} bytes] ${hex}`, true);
            });
          } else {
            const bytes = new Uint8Array(data);
            const hex = Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join(' ');
            addMessage("received", `[Binary: ${bytes.length} bytes] ${hex}`, true);
          }
        } else {
          addMessage("received", String(data), false);
        }
      };

      ws.onclose = (event) => {
        setIsConnected(false);
        setConnectionStatus("disconnected");
        addMessage("received", `WebSocket closed (code: ${event.code}, reason: ${event.reason})`, false);
      };

      ws.onerror = () => {
        setConnectionError("WebSocket error occurred");
        addMessage("received", "WebSocket error", false);
      };
    } catch (error) {
      setConnectionError(error instanceof Error ? error.message : "Connection failed");
      setConnectionStatus("disconnected");
    }
  };

  const disconnectWebSocket = () => {
    if (websocketRef.current) {
      websocketRef.current.close();
      websocketRef.current = null;
    }
  };

  const sendMessage = () => {
    if (!websocketRef.current || websocketRef.current.readyState !== WebSocket.OPEN) {
      return;
    }

    try {
      if (isBinary) {
        // Convert hex string to binary data
        const hexString = message.replace(/\s/g, "");
        const bytes = new Uint8Array(hexString.length / 2);
        for (let i = 0; i < hexString.length; i += 2) {
          bytes[i / 2] = parseInt(hexString.substr(i, 2), 16);
        }
        websocketRef.current.send(bytes);
        addMessage("sent", `[Binary: ${bytes.length} bytes] ${hexString}`, true);
      } else {
        websocketRef.current.send(message);
        addMessage("sent", message, false);
      }
    } catch (error) {
      addMessage("received", `Send error: ${error instanceof Error ? error.message : String(error)}`, false);
    }
  };

  const clearMessages = () => {
    setMessages([]);
  };

  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp).toLocaleTimeString();
  };

  // Update ping message timestamp
  const updatePingTimestamp = () => {
    if (message.includes('"timestamp"')) {
      try {
        const parsed = JSON.parse(message);
        parsed.timestamp = Date.now();
        setMessage(JSON.stringify(parsed, null, 2));
      } catch {
        // Not JSON, ignore
      }
    }
  };

  const exampleMessages = [
    { name: "Ping", data: '{"type": "ping", "timestamp": 0}', binary: false },
    { name: "Echo", data: '{"type": "echo", "message": "Hello WebSocket!"}', binary: false },
    { name: "Get Stats", data: '{"type": "getStats"}', binary: false },
    { name: "Binary Data", data: "48 65 6c 6c 6f", binary: true },
  ];

  const loadExample = (example: typeof exampleMessages[0]) => {
    if (example.name === "Ping") {
      const pingData = JSON.parse(example.data);
      pingData.timestamp = Date.now();
      setMessage(JSON.stringify(pingData, null, 2));
    } else {
      setMessage(example.data);
    }
    setIsBinary(example.binary);
  };

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      disconnectWebSocket();
    };
  }, []);

  return (
    <div>
      <div className="section">
        <h3>WebSocket Connection</h3>

        <div style={{ marginBottom: "15px" }}>
          <div style={{
            padding: "10px",
            border: "1px solid #ddd",
            borderRadius: "4px",
            background: isConnected ? "#d4edda" : "#f8d7da",
            color: isConnected ? "#155724" : "#721c24",
            marginBottom: "10px"
          }}>
            Status: {connectionStatus === "connecting" ? "🟡 Connecting..." :
                     isConnected ? "🟢 Connected" : "🔴 Disconnected"}
            {connectionError && ` - ${connectionError}`}
          </div>

          <div style={{
            fontSize: "13px",
            color: "#666",
            marginBottom: "10px"
          }}>
            Target: <code>{getWebSocketUrl()}</code>
          </div>

          <div style={{ display: "flex", gap: "10px" }}>
            <button
              className="btn btn-primary"
              onClick={connectWebSocket}
              disabled={isConnected || connectionStatus === "connecting"}
            >
              Connect
            </button>
            <button
              className="btn"
              onClick={disconnectWebSocket}
              disabled={!isConnected}
            >
              Disconnect
            </button>
          </div>
        </div>
      </div>

      <div className="section">
        <h3>Send Message</h3>

        <div className="form-group" style={{ marginBottom: "15px" }}>
          <label>Message Type:</label>
          <div className="toggle">
            <button
              className={!isBinary ? "active" : ""}
              onClick={() => setIsBinary(false)}
            >
              Text/JSON
            </button>
            <button
              className={isBinary ? "active" : ""}
              onClick={() => setIsBinary(true)}
            >
              Binary (Hex)
            </button>
          </div>
        </div>

        <div className="form-group" style={{ marginBottom: "15px" }}>
          <label>{isBinary ? "Binary Data (Hex):" : "Message (Text/JSON):"}</label>
          <textarea
            className="form-control textarea"
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            placeholder={isBinary ? "48 65 6c 6c 6f (Hello in hex)" : '{"type": "ping"}'}
            rows={4}
          />
        </div>

        <div style={{ display: "flex", gap: "10px", marginBottom: "15px" }}>
          <button
            className="btn btn-primary"
            onClick={sendMessage}
            disabled={!isConnected || !message.trim()}
          >
            Send
          </button>

          {!isBinary && message.includes('"timestamp"') && (
            <button
              className="btn"
              onClick={updatePingTimestamp}
            >
              Update Timestamp
            </button>
          )}
        </div>

        <div className="section">
          <h4>Example Messages</h4>
          <div style={{ display: "flex", gap: "10px", flexWrap: "wrap" }}>
            {exampleMessages.map(example => (
              <button
                key={example.name}
                className="btn"
                onClick={() => loadExample(example)}
                style={{ fontSize: "12px" }}
              >
                {example.name}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="section">
        <h3>Message History ({messages.length})</h3>

        <div style={{ marginBottom: "15px" }}>
          <button
            className="btn"
            onClick={clearMessages}
          >
            Clear History
          </button>
        </div>

        {messages.length === 0 ? (
          <div style={{ textAlign: "center", color: "#666", padding: "20px" }}>
            No messages yet. Connect and send a message to see the conversation.
          </div>
        ) : (
          <div className="event-list">
            {messages.map(msg => (
              <div key={msg.id} className="event-item">
                <span className="timestamp">{formatTimestamp(msg.timestamp)}</span>
                <span className={`name ${msg.type === "sent" ? "sent" : "received"}`}>
                  {msg.type === "sent" ? "→ SENT" : "← RECEIVED"}
                  {msg.isBinary && " (BINARY)"}
                </span>
                <div style={{ marginTop: "5px", color: "#333", fontFamily: "monospace", fontSize: "12px" }}>
                  {msg.data}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}