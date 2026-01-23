import { httpRouter } from "convex/server";
import { httpAction } from "./_generated/server";
import { api } from "./_generated/api";
import { addRivetRoutes } from "@rivetkit/convex";

const http = httpRouter();
addRivetRoutes(http, httpAction, api.rivet.handle);
export default http;
