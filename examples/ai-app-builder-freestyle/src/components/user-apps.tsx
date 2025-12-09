"use client";

import { useEffect, useState } from "react";
import { AppCard } from "./app-card";
import { client } from "@/rivet/client";
import type { AppInfo } from "@/rivet/registry";

export function UserApps() {
  const [apps, setApps] = useState<AppInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadApps();
  }, []);

  async function loadApps() {
    setIsLoading(true);
    try {
      // Get the appList actor handle and call getAppIds
      const appIds = await client.appList.getOrCreate(["global"]).getAppIds();

      // Fetch app info for each app
      const appInfos: AppInfo[] = [];
      for (const appId of appIds) {
        try {
          const info = await client.appStore.getOrCreate([appId]).getInfo();
          if (info) {
            appInfos.push(info);
          }
        } catch {
          // Skip apps that can't be loaded
        }
      }

      // Sort by createdAt desc
      appInfos.sort((a, b) => b.createdAt - a.createdAt);
      setApps(appInfos);
    } catch (err) {
      console.error("Failed to load apps:", err);
    }
    setIsLoading(false);
  }

  const handleDelete = () => {
    loadApps();
  };

  if (isLoading) {
    return (
      <div className="px-4 sm:px-8 text-muted-foreground">
        Loading apps...
      </div>
    );
  }

  if (apps.length === 0) {
    return (
      <div className="px-4 sm:px-8 text-muted-foreground">
        No apps yet. Create one above!
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4 px-4 sm:px-8">
      {apps.map((app) => (
        <AppCard
          key={app.id}
          id={app.id}
          name={app.name}
          createdAt={new Date(app.createdAt)}
          onDelete={handleDelete}
        />
      ))}
    </div>
  );
}
