import { useState } from "react";
import type { AppState } from "../../App";

interface TabProps {
  state: AppState;
  updateState: (updates: Partial<AppState>) => void;
  client: any;
  actorHandle: any;
}

export default function RawHttpTab({ state }: TabProps) {
  const [method, setMethod] = useState("GET");
  const [path, setPath] = useState("/api/hello");
  const [headers, setHeaders] = useState("{}");
  const [body, setBody] = useState("");
  const [response, setResponse] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  const parseHeaders = (headersString: string) => {
    try {
      return JSON.parse(headersString);
    } catch {
      return {};
    }
  };

  const getActorUrl = () => {
    const baseUrl = "http://localhost:6420";
    const actorPath = state.actorKey ?
      `/actors/${state.actorName}/${encodeURIComponent(state.actorKey)}` :
      `/actors/${state.actorName}`;
    return `${baseUrl}${actorPath}`;
  };

  const handleSendRequest = async () => {
    setIsLoading(true);
    setResponse("");

    try {
      const url = `${getActorUrl()}${path}`;
      const requestHeaders = parseHeaders(headers);

      const requestOptions: RequestInit = {
        method,
        headers: {
          "Content-Type": "application/json",
          ...requestHeaders,
        },
      };

      if (method !== "GET" && method !== "HEAD" && body) {
        requestOptions.body = body;
      }

      const startTime = Date.now();
      const res = await fetch(url, requestOptions);
      const endTime = Date.now();

      const responseHeaders = Object.fromEntries(res.headers.entries());
      const responseBody = await res.text();

      const responseData = {
        status: res.status,
        statusText: res.statusText,
        duration: `${endTime - startTime}ms`,
        headers: responseHeaders,
        body: responseBody,
        url: url,
      };

      setResponse(JSON.stringify(responseData, null, 2));
    } catch (error) {
      setResponse(`Error: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsLoading(false);
    }
  };

  const predefinedPaths = [
    "/api/hello",
    "/api/echo",
    "/api/stats",
    "/api/headers",
    "/api/json",
    "/api/custom?param=value",
  ];

  const exampleRequests = [
    {
      name: "Get Hello",
      method: "GET",
      path: "/api/hello",
      headers: "{}",
      body: "",
    },
    {
      name: "Echo POST",
      method: "POST",
      path: "/api/echo",
      headers: '{"Content-Type": "text/plain"}',
      body: "Hello from client!",
    },
    {
      name: "JSON POST",
      method: "POST",
      path: "/api/json",
      headers: '{"Content-Type": "application/json"}',
      body: '{"message": "Hello", "timestamp": 1234567890}',
    },
    {
      name: "Get Stats",
      method: "GET",
      path: "/api/stats",
      headers: "{}",
      body: "",
    },
  ];

  const loadExample = (example: typeof exampleRequests[0]) => {
    setMethod(example.method);
    setPath(example.path);
    setHeaders(example.headers);
    setBody(example.body);
  };

  return (
    <div>
      <div className="section">
        <h3>Raw HTTP Request Builder</h3>

        <div className="form-grid">
          <div className="form-group">
            <label>Method:</label>
            <select
              className="form-control"
              value={method}
              onChange={(e) => setMethod(e.target.value)}
            >
              <option value="GET">GET</option>
              <option value="POST">POST</option>
              <option value="PUT">PUT</option>
              <option value="DELETE">DELETE</option>
              <option value="PATCH">PATCH</option>
              <option value="HEAD">HEAD</option>
            </select>
          </div>

          <div className="form-group">
            <label>Path:</label>
            <select
              className="form-control"
              value={path}
              onChange={(e) => setPath(e.target.value)}
            >
              {predefinedPaths.map(p => (
                <option key={p} value={p}>{p}</option>
              ))}
            </select>
          </div>
        </div>

        <div className="form-group" style={{ marginBottom: "15px" }}>
          <label>Custom Path:</label>
          <input
            className="form-control"
            type="text"
            value={path}
            onChange={(e) => setPath(e.target.value)}
            placeholder="/api/custom"
          />
        </div>

        <div className="form-group" style={{ marginBottom: "15px" }}>
          <label>Headers (JSON):</label>
          <textarea
            className="form-control textarea"
            value={headers}
            onChange={(e) => setHeaders(e.target.value)}
            placeholder='{"Content-Type": "application/json"}'
            rows={3}
          />
        </div>

        {method !== "GET" && method !== "HEAD" && (
          <div className="form-group" style={{ marginBottom: "15px" }}>
            <label>Body:</label>
            <textarea
              className="form-control textarea"
              value={body}
              onChange={(e) => setBody(e.target.value)}
              placeholder="Request body..."
              rows={4}
            />
          </div>
        )}

        <div style={{ marginBottom: "15px" }}>
          <button
            className="btn btn-primary"
            onClick={handleSendRequest}
            disabled={isLoading}
          >
            {isLoading ? "Sending..." : "Send Request"}
          </button>
        </div>

        <div style={{
          background: "var(--bg-tertiary)",
          border: "1px solid var(--border-secondary)",
          borderRadius: "4px",
          padding: "10px",
          marginBottom: "15px",
          fontSize: "13px"
        }}>
          <strong>Target URL:</strong> <code>{getActorUrl()}{path}</code>
        </div>
      </div>

      <div className="section">
        <h3>Example Requests</h3>
        <div style={{ display: "flex", gap: "10px", flexWrap: "wrap", marginBottom: "15px" }}>
          {exampleRequests.map(example => (
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