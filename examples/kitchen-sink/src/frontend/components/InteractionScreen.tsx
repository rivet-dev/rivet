import { useState } from "react";
import type { AppState } from "../App";
import ActionsTab from "./tabs/ActionsTab";
import EventsTab from "./tabs/EventsTab";
import ScheduleTab from "./tabs/ScheduleTab";
import SleepTab from "./tabs/SleepTab";
import RawHttpTab from "./tabs/RawHttpTab";
import RawWebSocketTab from "./tabs/RawWebSocketTab";
import MetadataTab from "./tabs/MetadataTab";
import ConnectionsTab from "./tabs/ConnectionsTab";

interface InteractionScreenProps {
  state: AppState;
  updateState: (updates: Partial<AppState>) => void;
  client: any;
  actorHandle: any;
  onDisconnect: () => void;
}

export interface EventSubscription {
  eventName: string;
  unsubscribe: () => void;
}

export interface EventItem {
  timestamp: number;
  name: string;
  data: any;
  id: string;
}

type TabType = "actions" | "events" | "connections" | "schedule" | "sleep" | "raw-http" | "raw-websocket" | "metadata";

export default function InteractionScreen({
  state,
  updateState,
  client,
  actorHandle,
  onDisconnect
}: InteractionScreenProps) {
  const [activeTab, setActiveTab] = useState<TabType>("actions");
  const [isDisconnecting, setIsDisconnecting] = useState(false);
  const [eventSubscriptions, setEventSubscriptions] = useState<Map<string, EventSubscription>>(new Map());
  const [events, setEvents] = useState<EventItem[]>([]);

  // Switch to an enabled tab if current tab becomes disabled
  const isCurrentTabDisabled = activeTab === "events" && state.connectionMode !== "connection";
  if (isCurrentTabDisabled) {
    setActiveTab("actions");
  }

  const handleDisconnect = async () => {
    setIsDisconnecting(true);

    onDisconnect();
    setIsDisconnecting(false);
  };

  const [isDisposing, setIsDisposing] = useState(false);

  const handleDispose = async () => {
    setIsDisposing(true);

    try {
      const actorName = state.actorName as keyof typeof client;
      const accessor = client[actorName];
      const key = state.actorKey ? [state.actorKey] : [];
      await accessor.get(key).dispose();

      // After disposal, go back to connection screen
      onDisconnect();
    } catch (error) {
      console.error("Failed to dispose actor:", error);
      alert("Failed to dispose actor. See console for details.");
    } finally {
      setIsDisposing(false);
    }
  };

  const tabs = [
    { id: "actions" as const, label: "Actions", component: ActionsTab, disabled: false },
    { id: "events" as const, label: "Events", component: EventsTab, disabled: state.connectionMode !== "connection" },
    { id: "connections" as const, label: "Connections", component: ConnectionsTab, disabled: false },
    { id: "schedule" as const, label: "Schedule", component: ScheduleTab, disabled: false },
    { id: "sleep" as const, label: "Sleep", component: SleepTab, disabled: false },
    { id: "raw-http" as const, label: "Raw HTTP", component: RawHttpTab, disabled: false },
    { id: "raw-websocket" as const, label: "Raw WebSocket", component: RawWebSocketTab, disabled: false },
    { id: "metadata" as const, label: "Metadata", component: MetadataTab, disabled: false },
  ];

  const ActiveTabComponent = tabs.find(tab => tab.id === activeTab)?.component || ActionsTab;

  return (
    <div className="interaction-modal-overlay">
      <div className="interaction-modal">
        {/* Header with connection info and controls */}
        <div className="interaction-header">
          <div className="connection-info">
            <div className="connection-details">
              <h2>Connected to Actor</h2>
              <div className="connection-meta">
                <span className="actor-name">{state.actorName}</span>
                {state.actorKey && <span className="actor-key">#{state.actorKey}</span>}
                <span className="transport-info">{state.transport.toUpperCase()}/{state.encoding.toUpperCase()}</span>
                <span className={`mode-info ${state.connectionMode}`}>
                  {state.connectionMode === "connection" ? "🔗 Connected" : "🔧 Handle"}
                </span>
              </div>
            </div>

            <div className="connection-actions">
              <button
                className="btn"
                onClick={handleDispose}
                disabled={isDisposing}
                aria-busy={isDisposing ? "true" : "false"}
              >
                {isDisposing ? "Disposing..." : "Dispose"}
              </button>
              <button
                className="btn"
                onClick={handleDisconnect}
                disabled={isDisconnecting}
                aria-busy={isDisconnecting ? "true" : "false"}
              >
                {isDisconnecting ? "Disconnecting..." : "Disconnect"}
              </button>
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div className="tabs">
          <div className="tab-list">
            {tabs.map(tab => (
              <button
                key={tab.id}
                className={`tab-button ${activeTab === tab.id ? "active" : ""}`}
                onClick={() => setActiveTab(tab.id)}
                disabled={tab.disabled}
              >
                {tab.label}
              </button>
            ))}
          </div>

          <div className="tab-content">
            {activeTab === "events" ? (
              <EventsTab
                state={state}
                updateState={updateState}
                client={client}
                actorHandle={actorHandle}
                eventSubscriptions={eventSubscriptions}
                setEventSubscriptions={setEventSubscriptions}
                events={events}
                setEvents={setEvents}
              />
            ) : (
              <ActiveTabComponent
                state={state}
                updateState={updateState}
                client={client}
                actorHandle={actorHandle}
                {...({} as any)}
              />
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
