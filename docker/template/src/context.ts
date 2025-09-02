import * as fs from "fs";
import * as path from "path";
import type { TemplateConfig } from "./config";

export const CORE_NETWORK_NAME = "rivet-core-network";
// If there's only one datacenter
export const SIMPLE_NETWORK_NAME = "rivet-network";

export class TemplateContext {
	config: TemplateConfig;
	private outputDir: string;

	constructor(config: TemplateConfig, outputDir: string) {
		this.config = config;
		this.outputDir = outputDir;
	}

	getDatacenter(dcId: string) {
		const datacenter = this.config.datacenters.find((dc) => dc.name === dcId);
		if (!datacenter)
			throw new Error(`Datacenter ${dcId} not found in configuration`);
		return datacenter;
	}

	writeFile(filePath: string, content: string) {
		const fullPath = path.join(this.outputDir, filePath);
		const dirPath = path.dirname(fullPath);

		if (!fs.existsSync(dirPath)) {
			fs.mkdirSync(dirPath, { recursive: true });
		}

		fs.writeFileSync(fullPath, content);
	}

	getCoreServicePath(service: string): string {
		// If only one datacenter, put core services at root level
		if (this.config.datacenters.length === 1) {
			return service;
		}
		return `core/${service}`;
	}

	getDatacenterServicePath(
		service: string,
		dcId: string,
		nodeIndex?: number,
	): string {
		// If only one datacenter, put services at root level
		if (!this.shouldIncludeDatacenterInName()) {
			if (this.shouldIncludeIndexInPath(service, dcId, nodeIndex)) {
				return `${service}/${nodeIndex}`;
			} else {
				return service;
			}
		}

		// Multiple datacenters, use datacenter folder
		if (this.shouldIncludeIndexInPath(service, dcId, nodeIndex)) {
			return `datacenters/${dcId}/${service}/${nodeIndex}`;
		} else {
			return `datacenters/${dcId}/${service}`;
		}
	}

	writeCoreServiceFile(service: string, fileName: string, content: string) {
		const basePath = this.getCoreServicePath(service);
		const filePath = `${basePath}/${fileName}`;
		this.writeFile(filePath, content);
	}

	writeDatacenterServiceFile(
		service: string,
		dcId: string,
		fileName: string,
		content: string,
		nodeIndex?: number,
	) {
		const basePath = this.getDatacenterServicePath(service, dcId, nodeIndex);
		const filePath = `${basePath}/${fileName}`;
		this.writeFile(filePath, content);
	}

	makeDatabaseFileExec(
		service: string,
		dcId: string,
		fileName: string,
		nodeIndex?: number,
	) {
		const basePath = this.getDatacenterServicePath(service, dcId, nodeIndex);
		const filePath = `${basePath}/${fileName}`;
		const fullPath = path.join(this.outputDir, filePath);

		fs.chmodSync(fullPath, 0o755);
	}

	shouldIncludeDatacenterInName(): boolean {
		return this.config.datacenters.length > 1;
	}

	shouldIncludeIndexInName(
		service: string,
		dcId: string,
		index?: number,
	): boolean {
		if (index === undefined) {
			return false;
		}

		const datacenter = this.getDatacenter(dcId);

		// Determine max instances based on service type
		let maxInstances = 1;
		if (service === "rivet-engine" || service === "rivet-shell") {
			maxInstances = datacenter.engines;
		} else if (service === "runner") {
			maxInstances = datacenter.runners;
		}
		// postgres always has 1 instance per datacenter

		return maxInstances > 1;
	}

	shouldIncludeIndexInPath(
		service: string,
		dcId: string,
		nodeIndex?: number,
	): boolean {
		if (nodeIndex === undefined) {
			return false;
		}

		const datacenter = this.getDatacenter(dcId);

		// Determine max instances based on service type
		let maxInstances = 1;
		if (service === "rivet-engine") {
			maxInstances = datacenter.engines;
		} else if (service === "runner") {
			maxInstances = datacenter.runners;
		}
		// postgres always has 1 instance per datacenter

		return maxInstances > 1;
	}

	shouldIncludeIndexInVolume(
		service: string,
		dcId: string,
		index?: number,
	): boolean {
		if (index === undefined) {
			return false;
		}

		const datacenter = this.getDatacenter(dcId);

		// Determine max instances based on service type
		let maxInstances = 1;
		if (service === "runner") {
			maxInstances = datacenter.runners;
		}
		// postgres always has 1 instance per datacenter

		return maxInstances > 1;
	}

	getDatacenterNetworkName(dcId: string): string {
		// If only one datacenter, use simpler network name
		if (!this.shouldIncludeDatacenterInName()) {
			return SIMPLE_NETWORK_NAME;
		}
		return `rivet-network-${dcId}`;
	}

	getServiceName(service: string, dcId: string, index?: number): string {
		// Build service name based on conditions
		let name = service;
		if (this.shouldIncludeDatacenterInName()) {
			name += `-${dcId}`;
		}
		if (this.shouldIncludeIndexInName(service, dcId, index)) {
			name += `-${index}`;
		}
		return name;
	}

	// Returns the hostname to use for service-to-service communication.
	// In host networking, services communicate via 127.0.0.1; otherwise use the service DNS name.
	getServiceHost(service: string, dcId: string, index?: number): string {
		if ((this.config as any).networkMode === "host") return "127.0.0.1";
		return this.getServiceName(service, dcId, index);
	}

	getVolumeName(service: string, dcId: string, index?: number): string {
		// Build volume name based on conditions
		let name = `${service}-data`;
		if (this.shouldIncludeDatacenterInName()) {
			name += `-${dcId}`;
		}
		if (this.shouldIncludeIndexInVolume(service, dcId, index)) {
			name += `-${index}`;
		}
		return name;
	}
}
