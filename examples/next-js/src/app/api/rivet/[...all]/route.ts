import { toNextHandler } from "@rivetkit/next-js";
import { registry } from "@/rivet/actors";

export const maxDuration = 300;

export const { GET, POST, PUT, PATCH, HEAD, OPTIONS } = toNextHandler(registry);
