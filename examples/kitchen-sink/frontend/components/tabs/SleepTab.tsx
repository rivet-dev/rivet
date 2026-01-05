import { useState } from "react";
import type { AppState } from "../../App";

interface TabProps {
  state: AppState;
  updateState: (updates: Partial<AppState>) => void;
  client: any;
  actorHandle: any;
}

export default function SleepTab({ state, actorHandle }: TabProps) {
  const [response, setResponse] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  const callAction = async (actionName: string, args: any[] = []) => {
    return await actorHandle[actionName](...args);
  };

  const handleTriggerSleep = async () => {
    setIsLoading(true);
    setResponse("");

    try {
      const result = await callAction("triggerSleep");
      setResponse(JSON.stringify(result, null, 2));
    } catch (error) {
      setResponse(`Error: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleGetLifecycleInfo = async () => {
    setIsLoading(true);
    setResponse("");

    try {
      const result = await callAction("getLifecycleInfo");
      setResponse(JSON.stringify(result, null, 2));
    } catch (error) {
      setResponse(`Error: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsLoading(false);
    }
  };

  if (state.actorName !== "demo") {
    return (
      <div className="section">
        <h3>Sleep & Lifecycle</h3>
        <p>Sleep and lifecycle features are only available for the <code>demo</code> actor. Please select the demo actor in the header.</p>
      </div>
    );
  }

  return (
    <div>
      <div className="section">
        <h3>Force Sleep</h3>
        <p style={{ marginBottom: "15px" }}>
          The <code>sleep()</code> method forces an actor to go dormant immediately.
          This is useful for testing actor lifecycle and persistence.
        </p>

        <button
          className="btn btn-primary"
          onClick={handleTriggerSleep}
          disabled={isLoading}
        >
          {isLoading ? "Triggering..." : "Trigger Sleep"}
        </button>
      </div>

      <div className="section">
        <h3>Lifecycle Information</h3>
        <p style={{ marginBottom: "15px" }}>
          View how many times the actor has been started and stopped.
          This demonstrates the actor lifecycle across sleep/wake cycles.
        </p>

        <button
          className="btn"
          onClick={handleGetLifecycleInfo}
          disabled={isLoading}
        >
          Get Lifecycle Info
        </button>
      </div>

      {response && (
        <div className="section">
          <h3>Response</h3>
          <div className="response">
            {response}
          </div>
        </div>
      )}
    </div>
  );
}