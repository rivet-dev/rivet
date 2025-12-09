import { FreestyleSandboxes } from "freestyle-sandboxes";

export const freestyle = new FreestyleSandboxes({
  apiKey: process.env.FREESTYLE_API_KEY!,
});
