let buffer = "";
let nextSession = 1;
let activeCwd = null;

const scenario = process.env.MOCK_RESUME_SCENARIO || "native";
const cwdEnvProbe = process.env.MOCK_CWD_ENV_PROBE || "";

const modes = {
  currentModeId: "default",
  availableModes: [{ id: "default", label: "Default" }],
};

function write(value) {
  process.stdout.write(`${JSON.stringify(value)}\n`);
}

function response(id, result) {
  write({ jsonrpc: "2.0", id, result });
}

function error(id, code, message, data) {
  write({ jsonrpc: "2.0", id, error: { code, message, data } });
}

function notification(method, params) {
  write({ jsonrpc: "2.0", method, params });
}

function sessionId(prefix) {
  return `${prefix}-${process.pid}-${nextSession++}`;
}

process.stdin.resume();
process.stdin.on("data", (chunk) => {
  buffer += chunk instanceof Uint8Array ? new TextDecoder().decode(chunk) : String(chunk);

  while (true) {
    const newline = buffer.indexOf("\n");
    if (newline === -1) break;
    const line = buffer.slice(0, newline);
    buffer = buffer.slice(newline + 1);
    if (!line.trim()) continue;

    const msg = JSON.parse(line);
    switch (msg.method) {
      case "initialize": {
        const agentCapabilities = { promptCapabilities: {} };
        if (scenario !== "no-loadsession") agentCapabilities.loadSession = true;
        response(msg.id, {
          protocolVersion: 1,
          agentInfo: { name: "rivetkit-mock-opencode", version: "0.0.0-test" },
          agentCapabilities,
          modes,
        });
        break;
      }
      case "session/new":
        activeCwd = msg.params?.cwd ?? null;
        response(msg.id, { sessionId: sessionId("mock-live"), modes });
        break;
      case "session/load":
        activeCwd = msg.params?.cwd ?? null;
        if (scenario === "native") {
          response(msg.id, { modes });
        } else {
          error(msg.id, -32603, "Internal error", { details: "NotFoundError" });
        }
        break;
      case "session/prompt": {
        const sid = msg.params?.sessionId;
        const blocks = Array.isArray(msg.params?.prompt) ? msg.params.prompt : [];
        const outputBlocks = cwdEnvProbe
          ? [
              ...blocks,
              {
                type: "probe",
                text: JSON.stringify({
                  cwd: activeCwd,
                  env: cwdEnvProbe,
                }),
              },
            ]
          : blocks;
        notification("session/update", {
          sessionId: sid,
          update: {
            sessionUpdate: "tool_call",
            title: "fixture/read",
            status: "completed",
            content: [{ type: "content", content: { type: "text", text: "tool-call-captured" } }],
          },
        });
        notification("session/update", {
          sessionId: sid,
          update: {
            sessionUpdate: "agent_message_chunk",
            content: { type: "text", text: JSON.stringify(outputBlocks) },
          },
        });
        response(msg.id, { stopReason: "end_turn" });
        break;
      }
      case "session/cancel":
        response(msg.id, {});
        break;
      default:
        error(msg.id, -32601, "Method not found", { method: msg.method });
        break;
    }
  }
});
