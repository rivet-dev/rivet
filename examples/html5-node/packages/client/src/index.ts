import { Rivet } from "rivet-sdk";

const urlParams = new URLSearchParams(window.location.search);

let rivet: Rivet;
let currentWebSocket: WebSocket | null = null;

/**
 * Sets the endpoint for the Rivet client and updates the UI
 */
function setEndpoint(newEndpoint: string): void {
  rivet = new Rivet({ endpoint: newEndpoint });
  (document.getElementById("endpointInput") as HTMLInputElement).value = newEndpoint;
  localStorage.setItem("endpoint", newEndpoint);
}

/**
 * Updates the status message in the UI
 */
function updateStatus(status: string): void {
  console.log("Status update", status);
  document.getElementById("status")!.textContent = status;
}

/**
 * Fetches and populates the available regions in the UI
 */
async function listRegions(): Promise<void> {
  updateStatus("Fetching regions...");
  try {
    const regions = await rivet.lobbies.listRegions({});
    const regionSelect = document.getElementById("regionSelect") as HTMLSelectElement;
    // Clear existing options
    regionSelect.innerHTML = "";
    // Add new options
    regionSelect.innerHTML = regions.regions.map((r) =>
      `<option value="${r.slug}">${r.slug}</option>`
    ).join("");
    updateStatus("Idle");
  } catch (error) {
    console.error("Error fetching regions:", error);
    updateStatus(`Error fetching regions: ${(error as Error).message}`);
    (document.getElementById("regionSelect") as HTMLSelectElement).innerHTML =
      '<option value="">Failed to fetch regions</option>';
  }
}

/**
 * Establishes a WebSocket connection to the game server
 */
function connect(lobby: any, players: any[]): void {
  let protocol: string;
  let hostname: string;
  let port: number;
  if (lobby.backend.server) {
    protocol = lobby.backend.server.ports["game"].protocol;
    hostname = lobby.backend.server.ports["game"].publicHostname;
    port = lobby.backend.server.ports["game"].publicPort;
  } else if (lobby.backend.localDevelopment) {
    protocol = "http";
    hostname = lobby.backend.localDevelopment.ports["game"].hostname;
    port = lobby.backend.localDevelopment.ports["game"].port;
  } else {
    throw new Error("unknown backend");
  }

  // Update status fields
  document.getElementById("origin")!.textContent = `${protocol}://${hostname}:${port}`;

  updateStatus("Connecting");

  let pingSend: number | null = null;

  // Close the existing WebSocket connection if it exists
  if (currentWebSocket) {
    currentWebSocket.close();
  }

  const ws = new WebSocket(
    `${protocol}://${hostname}:${port}?token=${players[0].token}`,
  );
  currentWebSocket = ws;

  ws.onopen = () => {
    updateStatus("Initiating");
  };
  ws.onerror = (err: Event) => {
    const errorMessage = `WebSocket error: ${
      (err as ErrorEvent).message || "Unknown error"
    }`;
    updateStatus(errorMessage);
    console.error(errorMessage, err);
  };
  ws.onclose = () => {
    updateStatus("Closed");
  };
  ws.onmessage = (ev: MessageEvent) => {
    try {
      let [event, data] = JSON.parse(ev.data as string);
      if (event == "init") {
        updateStatus("Ready");

        document.getElementById("publicIp")!.textContent = data.publicIp ?? "[not available locally]";
        setInterval(() => {
          ws.send(JSON.stringify(["ping", 1]));
          pingSend = performance.now();
        }, 1000);
      } else if (event == "pong") {
        console.log(
          "ping rtt",
          `${(performance.now() - pingSend!).toFixed(2)}ms`,
        );
      } else if (event == "counter") {
        document.getElementById("counter")!.textContent = data;
      }
    } catch (error) {
      const parseErrorMessage = `Error parsing WebSocket message: ${
        (error as Error).message
      }`;
      updateStatus(parseErrorMessage);
      console.error(parseErrorMessage, error);
    }
  };
}

/**
 * Finds or creates a lobby and connects to it
 */
(globalThis as any).findOrCreateLobby = async function (): Promise<void> {
  const endpoint = (document.getElementById("endpointInput") as HTMLInputElement).value;

  const region = (document.getElementById("regionSelect") as HTMLSelectElement).value;
  const tags = {};
  updateStatus("Waiting for lobby");
  try {
    let res = await rivet.lobbies.findOrCreate({
      version: (document.getElementById("gameVersionInput") as HTMLInputElement).value,
      regions: [region],
      tags,
      players: [{}],

      createConfig: {
        region,
        tags,
        maxPlayers: 8,
        maxPlayersDirect: 8,
      },
    });

    let { lobby, players } = res;

    // Update status fields
    document.getElementById("lobbyId")!.textContent = lobby.id;
    document.getElementById("region")!.textContent = lobby.region.name;

    // Test lobby connection
    updateStatus("Connecting");
    try {
      connect(lobby, players);
    } catch (err) {
      updateStatus(`Failed to connect: ${(err as Error).message}`);
      console.warn("connection failed", err);
    }
  } catch (error) {
    console.error("Error finding or creating lobby:", error);
    updateStatus(`Error finding or creating lobby: ${(error as Error).message}`);
  }
};

function setGameVersion(newGameVersion: string): void {
  (document.getElementById("gameVersionInput") as HTMLInputElement).value = newGameVersion;
  localStorage.setItem("gameVersion", newGameVersion);
}

globalThis.addEventListener("load", ()  => {
  // Load endpoint
  const storedEndpoint = localStorage.getItem("endpoint") ?? "http://127.0.0.1:6420";
  if (storedEndpoint) {
    setEndpoint(storedEndpoint);
  } else {
    updateStatus("Please enter an endpoint");
  }

  // Load game version
  const storedGameVersion = localStorage.getItem("gameVersion") ?? "default";
  if (storedGameVersion) {
    setGameVersion(storedGameVersion);
  } else {
    setGameVersion("default");
  }

  // Update the event listener to use 'input' instead of 'change'
  (document.getElementById("endpointInput") as HTMLInputElement).addEventListener("input", async function () {
    const newEndpoint = (document.getElementById("endpointInput") as HTMLInputElement).value;
    setEndpoint(newEndpoint);

    // Clear regions before refetching
    (document.getElementById("regionSelect") as HTMLSelectElement).innerHTML = "";

    // Refetch regions with the new endpoint
    await listRegions();
  });

  (document.getElementById("gameVersionInput") as HTMLInputElement).addEventListener("input", function () {
    const newGameVersion = (document.getElementById("gameVersionInput") as HTMLInputElement).value;
    setGameVersion(newGameVersion);
  });

  listRegions();
});
