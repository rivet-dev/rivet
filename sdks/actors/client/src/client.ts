import { ActorHandle } from "./handle.ts";
import { ActorTags } from "../../common/src/utils.ts";
import { ActorsRequest } from "../../manager-protocol/src/mod.ts";
import { CreateRequest } from "../../manager-protocol/src/query.ts";

export class ActorClient {
	constructor(private readonly managerEndpoint: string) {
	}

	//withId(actorId: string): Promise<ActorHandle> {
	//	return unimplemented();
	//}

	// TODO: Add auth params
	async withTags(
		tags: ActorTags,
		create?: CreateRequest,
	): Promise<ActorHandle> {
		create = create ?? {
			tags,
			buildTags: {
				...tags,
				current: "true",
			},
		};

		const res = await fetch(`${this.managerEndpoint}/actors`, {
			method: "POST",
			// TODO: Import type from protocol
			body: JSON.stringify(
				{
					query: {
						getOrCreate: {
							tags,
							create,
						},
					},
				} satisfies ActorsRequest,
			),
		});
		if (!res.ok) {
			throw new Error(
				`Manager error (${res.statusText}):\n${await res.text()}`,
			);
		}
		// TODO: Import type from protocol
		const resJson: { endpoint: string } = await res.json();

		const handle = new ActorHandle(resJson.endpoint);
		handle.connect();

		return handle;
	}

	//create(request: CreateRequest): Promise<ActorHandle> {
	//	return unimplemented();
	//}
}
