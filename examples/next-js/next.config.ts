import type { NextConfig } from "next";

const nextConfig: NextConfig = {
	serverExternalPackages: ["@rivetkit/sqlite", "@rivetkit/sqlite-vfs"],
};

export default nextConfig;
