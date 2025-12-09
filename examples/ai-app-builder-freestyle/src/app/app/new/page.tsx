"use client";

import { useEffect, useState, useRef } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { client } from "@/rivet/client";
import { templates } from "@/lib/templates";
import { createApp as createAppAction } from "@/actions/create-app";

export default function NewAppPage() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [isCreating, setIsCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const hasStartedRef = useRef(false);

  const message = searchParams.get("message") || "";
  const templateId = searchParams.get("template") || "nextjs";

  useEffect(() => {
    if (!hasStartedRef.current) {
      hasStartedRef.current = true;
      createApp();
    }
  }, []);

  async function createApp() {
    if (isCreating) return;
    setIsCreating(true);

    try {
      // Validate template
      if (!templates[templateId]) {
        throw new Error(
          `Template ${templateId} not found. Available templates: ${Object.keys(templates).join(", ")}`
        );
      }

      // Generate a new app ID
      const appId = crypto.randomUUID();

      // Call the server action to create the app
      const data = await createAppAction({
        appId,
        templateId,
        initialMessage: message ? decodeURIComponent(message) : undefined,
      });

      // Add app to global list
      await client.appList.getOrCreate(["global"]).addApp(appId);

      // Redirect to the app
      router.push(`/app/${data.id}`);
    } catch (err) {
      console.error("Failed to create app:", err);
      setError(err instanceof Error ? err.message : "Failed to create app");
    }
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center min-h-screen p-4">
        <div className="text-red-500 mb-4">Error: {error}</div>
        <button
          onClick={() => router.push("/")}
          className="px-4 py-2 bg-primary text-primary-foreground rounded-md"
        >
          Go back home
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-col items-center justify-center min-h-screen p-4">
      <div className="animate-pulse">
        <div className="text-lg font-medium">Creating your app...</div>
        <div className="text-sm text-muted-foreground mt-2">
          Setting up {templates[templateId]?.name || templateId} template
        </div>
      </div>
    </div>
  );
}
