"use server";

import { freestyle } from "@/lib/freestyle";

export async function requestDevServer({ repoId }: { repoId: string }) {
  const {
    ephemeralUrl,
    devCommandRunning,
    installCommandRunning,
    codeServerUrl,
  } = await freestyle.requestDevServer({
    repoId: repoId,
  });

  return {
    ephemeralUrl,
    devCommandRunning: true,
    installCommandRunning: false,
    codeServerUrl,
  };
}
