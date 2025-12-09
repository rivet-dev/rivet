"use client";

import { useEffect, useState, useRef } from "react";
import { useParams } from "next/navigation";
import AppWrapper from "@/components/app-wrapper";
import { buttonVariants } from "@/components/ui/button";
import Link from "next/link";
import { client } from "@/rivet/client";
import type { AppInfo } from "@/rivet/registry";
import type { UIMessage } from "ai";
import { requestDevServer } from "@/actions/request-dev-server";

interface DevServerInfo {
  codeServerUrl: string;
  ephemeralUrl: string;
}

export default function AppPage() {
  const params = useParams();
  const id = params.id as string;

  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [messages, setMessages] = useState<UIMessage[]>([]);
  const [devServer, setDevServer] = useState<DevServerInfo | null>(null);
  const [streamStatus, setStreamStatus] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const appStoreConnectionRef = useRef<any>(null);
  const streamStateConnectionRef = useRef<any>(null);

  // Load app data on mount
  useEffect(() => {
    loadAppData();
  }, [id]);

  // Connect to actors for real-time updates
  useEffect(() => {
    if (!appInfo) return;
    let mounted = true;

    const setupConnections = async () => {
      try {
        // Connect to appStore for new messages
        const appStoreConnection = await client.appStore.getOrCreate([id]).connect();
        if (!mounted) return;
        appStoreConnectionRef.current = appStoreConnection;

        appStoreConnection.on("newMessage", (message: UIMessage) => {
          if (mounted) {
            setMessages((prev) => [...prev, message]);
          }
        });

        // Connect to streamState for status updates
        const streamConnection = await client.streamState.getOrCreate([id]).connect();
        if (!mounted) return;
        streamStateConnectionRef.current = streamConnection;

        streamConnection.on("abort", () => {
          if (mounted) setStreamStatus(null);
        });

        // Get initial stream status
        const status = await client.streamState.getOrCreate([id]).getStatus();
        if (mounted) setStreamStatus(status);
      } catch (err) {
        console.error("Failed to setup actor connections:", err);
      }
    };

    setupConnections();

    return () => {
      mounted = false;
      if (appStoreConnectionRef.current?.disconnect) {
        appStoreConnectionRef.current.disconnect();
      }
      if (streamStateConnectionRef.current?.disconnect) {
        streamStateConnectionRef.current.disconnect();
      }
    };
  }, [id, appInfo]);

  async function loadAppData() {
    try {
      const data = await client.appStore.getOrCreate([id]).getAll();

      if (!data.info) {
        setError("App not found");
        setIsLoading(false);
        return;
      }

      setAppInfo(data.info);
      setMessages(data.messages);

      // Get stream status
      const status = await client.streamState.getOrCreate([id]).getStatus();
      setStreamStatus(status);

      // Request dev server
      const devServerData = await requestDevServer({ repoId: data.info.gitRepo });
      setDevServer(devServerData);
      setIsLoading(false);
    } catch (err) {
      console.error("Failed to load app:", err);
      setError(err instanceof Error ? err.message : "Failed to load app");
      setIsLoading(false);
    }
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="animate-pulse text-lg">Loading app...</div>
      </div>
    );
  }

  if (error || !appInfo) {
    return <ProjectNotFound />;
  }

  if (!devServer) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="animate-pulse text-lg">Starting dev server...</div>
      </div>
    );
  }

  return (
    <AppWrapper
      key={appInfo.id}
      baseId={appInfo.baseId}
      codeServerUrl={devServer.codeServerUrl}
      appName={appInfo.name}
      initialMessages={messages}
      consoleUrl={devServer.ephemeralUrl + "/__console"}
      repo={appInfo.gitRepo}
      appId={appInfo.id}
      repoId={appInfo.gitRepo}
      domain={appInfo.previewDomain ?? undefined}
      running={streamStatus === "running"}
    />
  );
}

function ProjectNotFound() {
  return (
    <div className="text-center my-16">
      Project not found.
      <div className="flex justify-center mt-4">
        <Link className={buttonVariants()} href="/">
          Go back to home
        </Link>
      </div>
    </div>
  );
}
