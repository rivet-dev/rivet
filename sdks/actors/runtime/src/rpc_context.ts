import { Connection } from "./connection.ts";

export class RpcContext<ConnectionData> {
	// TODO: Make readonly
	connection?: Connection<ConnectionData>;
}
