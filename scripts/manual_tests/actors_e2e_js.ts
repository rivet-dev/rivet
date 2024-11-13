#!/usr/bin/env -S deno run --allow-net --allow-env --allow-read

// Import necessary modules
import { resolve } from "https://deno.land/std@0.114.0/path/mod.ts";
import { v4 as uuidv4 } from "https://deno.land/std@0.114.0/uuid/mod.ts";

// Constants
const ENDPOINT = Deno.env.get("RIVET_ENDPOINT") ?? "http://127.0.0.1:8080";
const BUILD = Deno.env.get("RIVET_BUILD") ??
  resolve(
    import.meta.dirname,
    "../../resources/default-builds/js/test-js-echo/index.js",
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
    // headers: { "Content-Type": "application/javascript" },
    body: buildContent,
  });

  await httpRequest("POST", `${ENDPOINT}/builds/${build}/complete`, {});

  return { buildId: build };
}

async function createActor(datacenterId: string, buildId: string) {
  const response = await httpRequest("POST", `${ENDPOINT}/actors`, {
    tags: {},
    datacenter: datacenterId,
    network: {
      mode: "host",
      ports: {
        http: { protocol: "tcp" },
      },
    },
    resources: { cpu: 1000, memory: 1000 },
    runtime: {
      build: buildId,
    },
  });
  return response.actor;
}

async function main() {
  let { buildId } = await uploadBuild();

  const datacenters = await listDatacenters();
  const actor = await createActor(datacenters[0].id, buildId);

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
}

await main();
