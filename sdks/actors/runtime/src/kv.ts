// import { StorageDriver } from "../../driver.ts";
// import { __GlobalDurableObjectT } from "./global_durable_object.ts";

// // TODO: Re-export KV API but with prefixes for all keys
// export class Storage {
// 	constructor(private readonly durableObject: __GlobalDurableObjectT) {}

// 	async get<V>(key: string): Promise<V | undefined> {
// 		const jsonRaw = await this.durableObject.storage.get<string>(buildStorageKey(key));
// 		if (jsonRaw) {
// 			return await JSON.parse(jsonRaw);
// 		} else {
// 			return undefined;
// 		}
// 	}
// 	async put<V>(key: string, value: V): Promise<void> {
// 		await this.durableObject.storage.put(key, JSON.stringify(value));
// 	}
// 	async delete(key: string): Promise<void> {
// 		await this.durableObject.storage.delete(buildStorageKey(key));
// 	}
// }

// /**
//  * Build a key from the actor's API that's namespaced to the storage.
//  *
//  * This allows us to store metadata on different keys.
//  */
// function buildStorageKey(key: string): string {
// 	return `storage:${key}`;
// }
