#!/usr/bin/env -S deno run --allow-net --allow-env --allow-read

// Import necessary modules
import { resolve } from "https://deno.land/std@0.114.0/path/mod.ts";
import { v4 as uuidv4 } from "https://deno.land/std@0.114.0/uuid/mod.ts";

// Constants
const ENDPOINT = Deno.env.get("RIVET_ENDPOINT") ?? "http://127.0.0.1:8080";
const BUILD = Deno.env.get("RIVET_BUILD") ??
  resolve(
    import.meta.dirname,
    "./fixtures/echo_http.js",
  );

// Helper function to make HTTP requests
async function httpRequest(method: string, url: string, body?: any) {
  console.log(`Request: ${method} ${url}\n${JSON.stringify(body)}`);

  const response = await fetch(url, {
    method,
    headers: { "Content-Type": "application/json" },
    body: body ? JSON.stringify(body) : undefined,
  });
  const responseText = await response.text();

  console.log(`Response: ${response.status}\n${responseText}`);

  if (!response.ok) {
    throw new Error(
      `HTTP status: ${response.status}\n\nBody: ${responseText}`,
    );
  }

  console.log();

  return JSON.parse(responseText);
}

async function listDatacenters() {
  const response = await httpRequest("GET", `${ENDPOINT}/datacenters`);
  return response.datacenters;
}

async function uploadBuild() {
  const buildContent = await Deno.readFile(BUILD);
  const contentLength = buildContent.length;

  const randomString = crypto.randomUUID().replace(/-/g, "").slice(0, 8);
  const { build, image_presigned_request } = await httpRequest(
    "POST",
    `${ENDPOINT}/builds/prepare`,
    {
      image_file: {
        content_length: contentLength,
        path: "index.js",
      },
      kind: "javascript",
      name: `build-${randomString}`,
      // TODO: Remove
      image_tag: `actor:${randomString}`,
    },
  );

  await fetch(image_presigned_request.url, {
    method: "PUT",
    body: buildContent,
  });

  await httpRequest("POST", `${ENDPOINT}/builds/${build}/complete`, {});

  return { buildId: build };
}

async function createactorwithtags() {
 //  const { builds } = await httpRequest("GET", `${ENDPOINT}/builds`, {
	//   tags: { current: true, build: "mybuild" },
 //  });
	//
 //  const createResponse = await httpRequest("POST", `${ENDPOINT}/actors`, {
 //    tags: {
	// 	pool: "foo"
	// },
 //    datacenter: "local",
 //    network: {
 //      mode: "host",
 //      ports: {
 //        http: { protocol: "tcp", routing: { host: {} } },
 //      },
 //    },
 //    resources: { cpu: 1000, memory: 1000 },
 //    runtime: {
 //      build: builds[0].id,
 //    },
 //  });

  const createResponse = await httpRequest("POST", `${ENDPOINT}/actors`, {
    tags: {
      service: "matchmaker"
    },
    buildTags: { service: "matchmaker", current: true },
    datacenter: "local",
    network: {
      mode: "host",
      ports: {
        http: { protocol: "tcp", routing: { host: {} } },
      },
    },
    resources: { cpu: 1000, memory: 1000 },
  });
}

async function publishnewactorversion() {
  // const { id } = await httpRequest("POST", `${ENDPOINT}/builds`, {
	 //  // ...
  // });
  //
  // const { id } = await httpRequest("POST", `${ENDPOINT}/actors/upgrade`, {
	 //  tags: { pool: "foo" },
	 //  buildId: id
  // });

  // Handled by the CLI
  const { id } = await httpRequest("POST", `${ENDPOINT}/builds`, {
    tags: { pool: "matchmaker" },
  });

  // TODO:
  await httpRequest("POST", `${ENDPOINT}/actors/upgrade`, {
    tags: { pool: "foo" },
    build: id,
  });
}

async function actorlifecycle() {
  await httpRequest("POST", `${ENDPOINT}/builds`, {
	  // ...
    name: "mybuild",
    version: "0.2.0", // optional
  });

  await httpRequest("POST", `${ENDPOINT}/actors`, {
    tags: {
      pool: "foo"
    },
    datacenter: "local",
    network: {
      mode: "host",
      ports: {
        http: { protocol: "tcp", routing: { host: {} } },
      },
    },
    resources: { cpu: 1000, memory: 1000 },
    runtime: {
      build: "mybuild",
      // version: "0.2.0",  //optional (can this be upgraded?)
    },
  });


  await httpRequest("POST", `${ENDPOINT}/builds`, {
	  // ...
    name: "mybuild",
    version: "0.3.0",
    makeCurrent: true,
  });
}

async function createActor(datacenterId: string, buildId: string) {
  const createResponse = await httpRequest("POST", `${ENDPOINT}/actors`, {
    tags: {},
    datacenter: "local",
    network: {
      mode: "host",
      ports: {
        http: { protocol: "tcp", routing: { host: {} } },
      },
    },
    resources: { cpu: 1000, memory: 1000 },
	build: "mybuild",
	buildRevion: "2024-10-5",
    // runtime: {
    //   build: buildId,
    // },
  });

  while (true) {
    const { actor } = await httpRequest(
      "GET",
      `${ENDPOINT}/actors/${createResponse.actor.id}`,
    );
    if (actor.network.ports.http.public_hostname != null) {
      return actor;
    } else {
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
  }
}

async function pingActor(actor) {
  while (true) {
    try {
      console.log("Pinging actor");

      const response = await fetch(
        `http://${actor.network.ports.http.public_hostname}:${actor.network.ports.http.public_port}`,
        {
          method: "POST",
          body: "foo",
        },
      );
      const responseBody = await response.text();
      // Validate the response
      if (responseBody === "foo") {
        console.log("Response validated successfully.");
      } else {
        console.error("Response validation failed.");
      }

      console.log();

      break;
    } catch (err) {
      console.log("Failed to ping actor:", err);
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
  }
}

async function destroyActor(actor) {
  await httpRequest("DELETE", `${ENDPOINT}/actors/${actor.id}`);
}

async function main() {
  let { buildId } = await uploadBuild();

  const datacenters = await listDatacenters();
  const actor = await createActor(datacenters[0].id, buildId);

  await pingActor(actor);

  await new Promise(resolve => setTimeout(resolve, 5000));

  await destroyActor(actor);
}

await main();
