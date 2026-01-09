import { useState } from "react";
import type { AppState } from "../../App.tsx";

interface TabProps {
  state: AppState;
  updateState: (updates: Partial<AppState>) => void;
  client: any;
  actorHandle: any;
}

interface ActionResponse {
  [key: string]: string;
}

export default function ActionsTab({ state, actorHandle }: TabProps) {
  const [responses, setResponses] = useState<ActionResponse>({});
  const [loading, setLoading] = useState<{ [key: string]: boolean }>({});

  // Demo actor state
  const [incrementAmount, setIncrementAmount] = useState("1");
  const [message, setMessage] = useState("Hello World");
  const [delayAmount, setDelayAmount] = useState("3");
  const [delayMs, setDelayMs] = useState("2000");
  const [eventName, setEventName] = useState("userAction");
  const [eventData, setEventData] = useState('{"type": "click", "target": "button"}');

  // WebSocket actor state
  const [messageLimit, setMessageLimit] = useState("10");

  const callAction = async (actionName: string, ...args: any[]) => {
    setLoading((prev) => ({ ...prev, [actionName]: true }));
    setResponses((prev) => ({ ...prev, [actionName]: "" }));

    try {
      const result = await actorHandle[actionName](...args);
      setResponses((prev) => ({
        ...prev,
        [actionName]: JSON.stringify(result, null, 2),
      }));
    } catch (error) {
      setResponses((prev) => ({
        ...prev,
        [actionName]: `Error: ${error instanceof Error ? error.message : String(error)}`,
      }));
    } finally {
      setLoading((prev) => ({ ...prev, [actionName]: false }));
    }
  };

  const ActionSection = ({
    title,
    description,
    actionName,
    children,
    onCall,
  }: {
    title: string;
    description: string;
    actionName: string;
    children?: React.ReactNode;
    onCall: () => void;
  }) => (
    <div className="section">
      <h3>{title}</h3>
      <p style={{ fontSize: "14px", color: "#666", marginBottom: "10px" }}>
        {description}
      </p>
      {children}
      <button
        className="btn btn-primary"
        onClick={onCall}
        disabled={loading[actionName]}
        style={{ marginTop: children ? "10px" : "0" }}
      >
        {loading[actionName] ? "Calling..." : `Call ${title}`}
      </button>
      {responses[actionName] && (
        <div className="response" style={{ marginTop: "10px" }}>
          {responses[actionName]}
        </div>
      )}
    </div>
  );

  const renderDemoActions = () => (
    <>
      <ActionSection
        title="getCount"
        description="Get current counter value"
        actionName="getCount"
        onCall={() => callAction("getCount")}
      />

      <ActionSection
        title="increment"
        description="Increment counter by amount"
        actionName="increment"
        onCall={() => callAction("increment", Number(incrementAmount))}
      >
        <div className="form-group">
          <label>Amount:</label>
          <input
            className="form-control"
            type="number"
            value={incrementAmount}
            onChange={(e) => setIncrementAmount(e.target.value)}
          />
        </div>
      </ActionSection>

      <ActionSection
        title="setMessage"
        description="Set a message string"
        actionName="setMessage"
        onCall={() => callAction("setMessage", message)}
      >
        <div className="form-group">
          <label>Message:</label>
          <input
            className="form-control"
            type="text"
            value={message}
            onChange={(e) => setMessage(e.target.value)}
          />
        </div>
      </ActionSection>

      <ActionSection
        title="delayedIncrement"
        description="Increment after delay"
        actionName="delayedIncrement"
        onCall={() => callAction("delayedIncrement", Number(delayAmount), Number(delayMs))}
      >
        <div className="form-grid">
          <div className="form-group">
            <label>Amount:</label>
            <input
              className="form-control"
              type="number"
              value={delayAmount}
              onChange={(e) => setDelayAmount(e.target.value)}
            />
          </div>
          <div className="form-group">
            <label>Delay (ms):</label>
            <input
              className="form-control"
              type="number"
              value={delayMs}
              onChange={(e) => setDelayMs(e.target.value)}
            />
          </div>
        </div>
      </ActionSection>

      <ActionSection
        title="promiseAction"
        description="Returns a resolved promise with timestamp"
        actionName="promiseAction"
        onCall={() => callAction("promiseAction")}
      />

      <ActionSection
        title="getState"
        description="Get both actor and connection state"
        actionName="getState"
        onCall={() => callAction("getState")}
      />

      <ActionSection
        title="getLifecycleInfo"
        description="Get start/stop counts"
        actionName="getLifecycleInfo"
        onCall={() => callAction("getLifecycleInfo")}
      />

      <ActionSection
        title="getMetadata"
        description="Get actor metadata"
        actionName="getMetadata"
        onCall={() => callAction("getMetadata")}
      />

      <ActionSection
        title="getInput"
        description="Get input data passed during actor creation"
        actionName="getInput"
        onCall={() => callAction("getInput")}
      />

      <ActionSection
        title="broadcastCustomEvent"
        description="Broadcast custom event"
        actionName="broadcastCustomEvent"
        onCall={() => {
          try {
            const data = JSON.parse(eventData);
            callAction("broadcastCustomEvent", eventName, data);
          } catch (error) {
            setResponses((prev) => ({
              ...prev,
              broadcastCustomEvent: `Error: Invalid JSON data`,
            }));
          }
        }}
      >
        <div className="form-grid">
          <div className="form-group">
            <label>Event Name:</label>
            <input
              className="form-control"
              type="text"
              value={eventName}
              onChange={(e) => setEventName(e.target.value)}
            />
          </div>
          <div className="form-group">
            <label>Data (JSON):</label>
            <input
              className="form-control"
              type="text"
              value={eventData}
              onChange={(e) => setEventData(e.target.value)}
            />
          </div>
        </div>
      </ActionSection>
    </>
  );

  const renderHttpActions = () => (
    <>
      <ActionSection
        title="getStats"
        description="Get HTTP request statistics"
        actionName="getStats"
        onCall={() => callAction("getStats")}
      />

      <ActionSection
        title="clearHistory"
        description="Clear request history"
        actionName="clearHistory"
        onCall={() => callAction("clearHistory")}
      />
    </>
  );

  const renderWebSocketActions = () => (
    <>
      <ActionSection
        title="getStats"
        description="Get WebSocket statistics"
        actionName="getStats"
        onCall={() => callAction("getStats")}
      />

      <ActionSection
        title="clearHistory"
        description="Clear message history"
        actionName="clearHistory"
        onCall={() => callAction("clearHistory")}
      />

      <ActionSection
        title="getMessageHistory"
        description="Get recent WebSocket messages"
        actionName="getMessageHistory"
        onCall={() => callAction("getMessageHistory", Number(messageLimit))}
      >
        <div className="form-group">
          <label>Limit:</label>
          <input
            className="form-control"
            type="number"
            value={messageLimit}
            onChange={(e) => setMessageLimit(e.target.value)}
          />
        </div>
      </ActionSection>
    </>
  );

  return (
    <div>
      {state.actorName === "demo" && renderDemoActions()}
      {state.actorName === "http" && renderHttpActions()}
      {state.actorName === "websocket" && renderWebSocketActions()}
    </div>
  );
}