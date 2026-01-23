/**
 * Node.js setup - injects polyfills and all Node dependencies.
 *
 * This file is imported for side effects by node-entry.ts.
 * It sets up the Node.js environment before the rest of rivetkit loads.
 */
import { setNodeDependencies } from "./utils/node";

// 1. Inject Web API polyfills into globalThis
import { EventSource as EventSourcePkg } from "eventsource";
import WebSocketPkg from "ws";

if (typeof globalThis.EventSource === "undefined") {
	(globalThis as any).EventSource = EventSourcePkg;
}
if (typeof globalThis.WebSocket === "undefined") {
	(globalThis as any).WebSocket = WebSocketPkg;
}

// 2. Import all Node.js built-in modules
import * as fs from "node:fs";
import * as fsPromises from "node:fs/promises";
import * as path from "node:path";
import * as os from "node:os";
import * as childProcess from "node:child_process";
import * as crypto from "node:crypto";
import * as stream from "node:stream/promises";

// 3. Import Node-only npm packages
import getPort from "get-port";
import * as honoNodeServer from "@hono/node-server";
import * as honoNodeWs from "@hono/node-ws";

// 4. Inject all dependencies
setNodeDependencies({
	fs,
	fsPromises,
	path,
	os,
	childProcess,
	crypto,
	stream,
	getPort,
	honoNodeServer,
	honoNodeWs,
});
