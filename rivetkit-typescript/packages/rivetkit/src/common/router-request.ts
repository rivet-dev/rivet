import type { HonoRequest } from "hono";
import * as errors from "@/actor/errors";
import { HEADER_ENCODING } from "@/common/actor-router-consts";
import { getEnvUniversal } from "@/utils";
import { type Encoding, EncodingSchema } from "./encoding";

export function getRequestEncoding(req: HonoRequest): Encoding {
	const encodingParam = req.header(HEADER_ENCODING);
	if (!encodingParam) {
		return "json";
	}

	const result = EncodingSchema.safeParse(encodingParam);
	if (!result.success) {
		throw errors.invalidEncoding(encodingParam as string);
	}

	return result.data;
}

export function getRequestExposeInternalError(_req: Request): boolean {
	return getEnvUniversal("RIVET_EXPOSE_ERRORS") === "1";
}
