import { useState } from "react";
import type { AppState } from "../../App";

interface TabProps {
  state: AppState;
  updateState: (updates: Partial<AppState>) => void;
  client: any;
  actorHandle: any;
}

interface AlarmEntry {
  id: string;
  scheduledFor: number;
  data?: any;
  status: "scheduled" | "triggered";
}

export default function ScheduleTab({ state, actorHandle }: TabProps) {
  const [atTimestamp, setAtTimestamp] = useState("");
  const [afterDelay, setAfterDelay] = useState("5000");
  const [alarmData, setAlarmData] = useState("{}");
  const [response, setResponse] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [alarmHistory, setAlarmHistory] = useState<AlarmEntry[]>([]);

  const getCurrentTimestamp = () => {
    const now = new Date();
    const offset = now.getTimezoneOffset() * 60000;
    const localTime = new Date(now.getTime() - offset);
    return localTime.toISOString().slice(0, 16);
  };

  const parseAlarmData = (dataString: string) => {
    try {
      return JSON.parse(dataString);
    } catch {
      return {};
    }
  };

  const callAction = async (actionName: string, args: any[]) => {
    return await actorHandle[actionName](...args);
  };

  const handleScheduleAt = async () => {
    if (!atTimestamp) return;

    setIsLoading(true);
    setResponse("");

    try {
      const timestamp = new Date(atTimestamp).getTime();
      const data = parseAlarmData(alarmData);

      const result = await callAction("scheduleAlarmAt", [timestamp, data]);
      setResponse(JSON.stringify(result, null, 2));

      // Add to local tracking
      setAlarmHistory(prev => [...prev, {
        id: result.id,
        scheduledFor: result.scheduledFor,
        data,
        status: "scheduled",
      }]);
    } catch (error) {
      setResponse(`Error: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleScheduleAfter = async () => {
    const delay = parseInt(afterDelay);
    if (isNaN(delay) || delay < 0) return;

    setIsLoading(true);
    setResponse("");

    try {
      const data = parseAlarmData(alarmData);

      const result = await callAction("scheduleAlarmAfter", [delay, data]);
      setResponse(JSON.stringify(result, null, 2));

      // Add to local tracking
      setAlarmHistory(prev => [...prev, {
        id: result.id,
        scheduledFor: result.scheduledFor,
        data,
        status: "scheduled",
      }]);
    } catch (error) {
      setResponse(`Error: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleGetHistory = async () => {
    setIsLoading(true);
    setResponse("");

    try {
      const result = await callAction("getAlarmHistory", []);
      setResponse(JSON.stringify(result, null, 2));

      // Update status of triggered alarms
      const triggeredIds = result.map((entry: any) => entry.id);
      setAlarmHistory(prev => prev.map(alarm => ({
        ...alarm,
        status: triggeredIds.includes(alarm.id) ? "triggered" : alarm.status,
      })));
    } catch (error) {
      setResponse(`Error: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleClearHistory = async () => {
    setIsLoading(true);
    setResponse("");

    try {
      const result = await callAction("clearAlarmHistory", []);
      setResponse(JSON.stringify(result, null, 2));
      setAlarmHistory([]);
    } catch (error) {
      setResponse(`Error: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const formatTime = (timestamp: number) => {
    return new Date(timestamp).toLocaleString();
  };

  if (state.actorName !== "demo") {
    return (
      <div className="section">
        <h3>Scheduling</h3>
        <p>Scheduling features are only available for the <code>demo</code> actor. Please select the demo actor in the header.</p>
      </div>
    );
  }

  return (
    <div>
      <div className="section">
        <h3>Schedule Alarm At Specific Time</h3>
        <div className="form-grid">
          <div className="form-group">
            <label>Timestamp:</label>
            <input
              className="form-control"
              type="datetime-local"
              value={atTimestamp}
              onChange={(e) => setAtTimestamp(e.target.value)}
              min={getCurrentTimestamp()}
            />
          </div>

          <div className="form-group">
            <label>Alarm Data (JSON):</label>
            <input
              className="form-control"
              type="text"
              value={alarmData}
              onChange={(e) => setAlarmData(e.target.value)}
              placeholder='{"message": "Hello"}'
            />
          </div>

          <button
            className="btn btn-primary"
            onClick={handleScheduleAt}
            disabled={isLoading || !atTimestamp}
          >
            schedule.at()
          </button>
        </div>
      </div>

      <div className="section">
        <h3>Schedule Alarm After Delay</h3>
        <div className="form-grid">
          <div className="form-group">
            <label>Delay (ms):</label>
            <input
              className="form-control"
              type="number"
              value={afterDelay}
              onChange={(e) => setAfterDelay(e.target.value)}
              min="0"
              step="1000"
            />
          </div>

          <div className="form-group">
            <label>Alarm Data (JSON):</label>
            <input
              className="form-control"
              type="text"
              value={alarmData}
              onChange={(e) => setAlarmData(e.target.value)}
              placeholder='{"message": "Hello"}'
            />
          </div>

          <button
            className="btn btn-primary"
            onClick={handleScheduleAfter}
            disabled={isLoading}
          >
            schedule.after()
          </button>
        </div>
      </div>

      <div className="section">
        <h3>Alarm Management</h3>
        <div style={{ display: "flex", gap: "10px", marginBottom: "15px" }}>
          <button
            className="btn"
            onClick={handleGetHistory}
            disabled={isLoading}
          >
            Get Alarm History
          </button>
          <button
            className="btn"
            onClick={handleClearHistory}
            disabled={isLoading}
          >
            Clear History
          </button>
        </div>

        {alarmHistory.length > 0 && (
          <div>
            <h4>Scheduled Alarms</h4>
            <div style={{ marginBottom: "15px" }}>
              {alarmHistory.map(alarm => (
                <div
                  key={alarm.id}
                  style={{
                    padding: "10px",
                    border: "1px solid #ddd",
                    borderRadius: "4px",
                    marginBottom: "5px",
                    background: alarm.status === "triggered" ? "#d4edda" : "#fff3cd",
                  }}
                >
                  <strong>{alarm.id}</strong> - {alarm.status === "triggered" ? "✅ Triggered" : "⏰ Scheduled"}
                  <br />
                  Scheduled for: {formatTime(alarm.scheduledFor)}
                  {alarm.data && Object.keys(alarm.data).length > 0 && (
                    <>
                      <br />
                      Data: {JSON.stringify(alarm.data)}
                    </>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        {response && (
          <div className="response">
            {response}
          </div>
        )}
      </div>
    </div>
  );
}