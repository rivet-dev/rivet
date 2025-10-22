//import { exec as execCallback } from "node:child_process";
//import { setupLogging } from "@/common//log";
//import type { ClientOptions } from "./client";
//import { InternalError } from "./errors";
//import { Client } from "./mod.ts";
//
///**
// * Uses the Rivet CLI to read the manager endpoint to connect to. This allows
// * for writing tests that run locally without hardcoding the manager endpoint.
// */
//export async function readEndpointFromCli(): Promise<string> {
//	// Read endpoint
//	const cliPath = process.env.RIVET_CLI_PATH ?? "rivet";
//
//	try {
//		const { stdout, stderr } = await new Promise<{
//			stdout: string;
//			stderr: string;
//		}>((resolve, reject) => {
//			execCallback(`${cliPath} manager endpoint`, (error, stdout, stderr) => {
//				if (error) reject(error);
//				else resolve({ stdout, stderr });
//			});
//		});
//
//		if (stderr) {
//			throw new Error(stderr);
//		}
//
//		// Decode output
//		return stdout.trim();
//	} catch (error) {
//		throw new InternalError(`Read endpoint failed: ${error}`);
//	}
//}
//
//export class TestClient extends Client {
//	public constructor(opts?: ClientOptions) {
//		// Setup logging automatically
//		setupLogging();
//
//		super(readEndpointFromCli(), opts);
//	}
//}
