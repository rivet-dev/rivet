export interface VersionedData<T> {
	version: number;
	data: T;
}

export type MigrationFn<TFrom, TTo> = (data: TFrom) => TTo;

export interface VersionedDataConfig<T> {
	currentVersion: number;
	migrations: Map<number, MigrationFn<any, any>>;
	serializeVersion: (data: T) => Uint8Array;
	deserializeVersion: (bytes: Uint8Array) => T;
}

export class VersionedDataHandler<T> {
	constructor(private config: VersionedDataConfig<T>) {}

	serializeWithEmbeddedVersion(data: T): Uint8Array {
		const versioned: VersionedData<Uint8Array> = {
			version: this.config.currentVersion,
			data: this.config.serializeVersion(data),
		};

		return this.embedVersion(versioned);
	}

	deserializeWithEmbeddedVersion(bytes: Uint8Array): T {
		const versioned = this.extractVersion(bytes);
		return this.deserialize(versioned.data, versioned.version);
	}

	serialize(data: T, version: number): Uint8Array {
		return this.config.serializeVersion(data);
	}

	deserialize(bytes: Uint8Array, version: number): T {
		if (version === this.config.currentVersion) {
			return this.config.deserializeVersion(bytes);
		}

		if (version > this.config.currentVersion) {
			throw new Error(
				`Cannot decode data from version ${version}, current version is ${this.config.currentVersion}`,
			);
		}

		let currentData: any = this.config.deserializeVersion(bytes);
		let currentVersion = version;

		while (currentVersion < this.config.currentVersion) {
			const migration = this.config.migrations.get(currentVersion);
			if (!migration) {
				throw new Error(
					`No migration found from version ${currentVersion} to ${currentVersion + 1}`,
				);
			}

			currentData = migration(currentData);
			currentVersion++;
		}

		return currentData;
	}

	private embedVersion(data: VersionedData<Uint8Array>): Uint8Array {
		const versionBytes = new Uint8Array(2);
		new DataView(versionBytes.buffer).setUint16(0, data.version, true);

		const result = new Uint8Array(versionBytes.length + data.data.length);
		result.set(versionBytes);
		result.set(data.data, versionBytes.length);

		return result;
	}

	private extractVersion(bytes: Uint8Array): VersionedData<Uint8Array> {
		if (bytes.length < 2) {
			throw new Error("Invalid versioned data: too short");
		}

		const version = new DataView(bytes.buffer, bytes.byteOffset).getUint16(
			0,
			true,
		);
		const data = bytes.slice(2);

		return { version, data };
	}
}

export function createVersionedDataHandler<T>(
	config: VersionedDataConfig<T>,
): VersionedDataHandler<T> {
	return new VersionedDataHandler(config);
}
