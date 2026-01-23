import { useState, useRef } from "react";
import type { AppState } from "../../App.tsx";
import type { EventSubscription, EventItem } from "../InteractionScreen.tsx";

interface TabProps {
  state: AppState;
  updateState: (updates: Partial<AppState>) => void;
  client: any;
  actorHandle: any;
  eventSubscriptions: Map<string, EventSubscription>;
  setEventSubscriptions: React.Dispatch<React.SetStateAction<Map<string, EventSubscription>>>;
  events: EventItem[];
  setEvents: React.Dispatch<React.SetStateAction<EventItem[]>>;
}

export default function EventsTab({
  actorHandle,
  eventSubscriptions,
  setEventSubscriptions,
  events,
  setEvents
}: TabProps) {
  const [selectedEventType, setSelectedEventType] = useState("countChanged");
  const [customEventName, setCustomEventName] = useState("");
  const eventIdCounter = useRef(0);

  const predefinedEvents = [
    "countChanged",
    "messageChanged",
    "preferenceChanged",
    "alarmTriggered",
  ];

  const addEvent = (name: string, data: any) => {
    const event: EventItem = {
      timestamp: Date.now(),
      name,
      data,
      id: `event-${++eventIdCounter.current}`,
    };
    setEvents(prev => [...prev, event].slice(-100)); // Keep last 100 events
  };

  const subscribe = (eventName: string) => {
    if (!actorHandle || eventSubscriptions.has(eventName)) return;

    const unsubscribe = actorHandle.on(eventName, (...args: any[]) => {
      addEvent(eventName, args.length === 1 ? args[0] : args);
    });

    setEventSubscriptions(prev => {
      const next = new Map(prev);
      next.set(eventName, { eventName, unsubscribe });
      return next;
    });
  };

  const unsubscribe = (eventName: string) => {
    const subscription = eventSubscriptions.get(eventName);
    if (!subscription) return;

    subscription.unsubscribe();
    setEventSubscriptions(prev => {
      const next = new Map(prev);
      next.delete(eventName);
      return next;
    });
  };

  const handleSubscribe = () => {
    const eventName = selectedEventType === "custom" ? customEventName.trim() : selectedEventType;
    if (!eventName) return;

    subscribe(eventName);

    // Reset custom input if it was used
    if (selectedEventType === "custom") {
      setCustomEventName("");
    }
  };

  const clearEvents = () => {
    setEvents([]);
  };

  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp).toLocaleTimeString();
  };

  const formatData = (data: any) => {
    if (typeof data === "string") return data;
    return JSON.stringify(data, null, 2);
  };

  return (
    <div>
      <div className="section">
        <h3>Subscribe to Events</h3>

        <div style={{ display: "flex", gap: "10px", marginBottom: "15px" }}>
          <div className="form-group" style={{ flex: 1, marginBottom: 0 }}>
            <label>Event Name:</label>
            <select
              className="form-control"
              value={selectedEventType}
              onChange={(e) => setSelectedEventType(e.target.value)}
            >
              {predefinedEvents.map(event => (
                <option key={event} value={event}>{event}</option>
              ))}
              <option value="custom">Custom...</option>
            </select>
          </div>

          {selectedEventType === "custom" && (
            <div className="form-group" style={{ flex: 1, marginBottom: 0 }}>
              <label>Custom Event Name:</label>
              <input
                className="form-control"
                type="text"
                value={customEventName}
                onChange={(e) => setCustomEventName(e.target.value)}
                placeholder="Enter event name..."
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleSubscribe();
                }}
              />
            </div>
          )}

          <div style={{ display: "flex", alignItems: "flex-end" }}>
            <button
              className="btn btn-primary"
              onClick={handleSubscribe}
              disabled={selectedEventType === "custom" && !customEventName.trim()}
            >
              Subscribe
            </button>
          </div>
        </div>
      </div>

      <div className="section">
        <h3>Active Subscriptions ({eventSubscriptions.size})</h3>

        {eventSubscriptions.size === 0 ? (
          <div style={{ textAlign: "center", color: "var(--text-secondary)", padding: "20px" }}>
            No active subscriptions
          </div>
        ) : (
          <div style={{
            display: "flex",
            flexWrap: "wrap",
            gap: "8px"
          }}>
            {Array.from(eventSubscriptions.values()).map(sub => (
              <div
                key={sub.eventName}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                  background: "var(--bg-tertiary)",
                  border: "1px solid var(--border-secondary)",
                  borderRadius: "20px",
                  padding: "6px 12px",
                  fontSize: "14px"
                }}
              >
                <span style={{ fontFamily: "monospace" }}>{sub.eventName}</span>
                <button
                  onClick={() => unsubscribe(sub.eventName)}
                  style={{
                    background: "none",
                    border: "none",
                    color: "var(--danger-color)",
                    cursor: "pointer",
                    fontSize: "16px",
                    padding: 0,
                    lineHeight: 1,
                    width: "16px",
                    height: "16px",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center"
                  }}
                  title="Unsubscribe"
                >
                  ×
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="section">
        <h3>Event History ({events.length} events)</h3>

        <div style={{ marginBottom: "15px" }}>
          <button
            className="btn"
            onClick={clearEvents}
          >
            Clear ({events.length})
          </button>
        </div>

        {events.length === 0 ? (
          <div style={{ textAlign: "center", color: "var(--text-secondary)", padding: "20px" }}>
            No events received yet
          </div>
        ) : (
          <div className="event-list">
            {events.map(event => (
              <div key={event.id} className="event-item">
                <span className="timestamp">{formatTimestamp(event.timestamp)}</span>
                <span className="name">{event.name}</span>
                <div style={{ marginTop: "5px" }}>
                  {formatData(event.data)}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}