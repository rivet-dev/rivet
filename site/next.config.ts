import path from "node:path";
import url from "node:url";
import nextMDX from "@next/mdx";
import { config } from "./src/mdx/mdx";

const withMDX = nextMDX(config);

/** @type {import('next').NextConfig} */
const nextConfig = {
	output: "export",
	trailingSlash: true, // Required for Vercel
	reactStrictMode: true,
	transpilePackages: ["@rivet-gg/components", "@rivet-gg/icons"],
	typescript: {
		ignoreBuildErrors: true,
	},
	pageExtensions: ["js", "jsx", "ts", "tsx", "mdx", "md"],
	images: {
		// For static output
		unoptimized: true,
	},
	experimental: {
		scrollRestoration: true,
	},
	async redirects() {
		return [
			// Discord redirect
			{
				source: '/discord',
				destination: 'https://discord.gg/aXYfyNxYVn',
				permanent: false,
			},
			{
				source: '/discord/',
				destination: 'https://discord.gg/aXYfyNxYVn',
				permanent: false,
			},

			// Convenience Redirects
			{
				source: '/docs',
				destination: '/docs/actors',
				permanent: false,
			},

			// Redirects for moved Cloud docs
			{
				source: '/docs/actors-low-level',
				destination: '/docs/cloud/actors',
				permanent: false,
			},
			{
				source: '/docs/api',
				destination: '/docs/cloud/api',
				permanent: false,
			},
			{
				source: '/docs/api/:slug*',
				destination: '/docs/cloud/api/:slug*',
				permanent: false,
			},
			{
				source: '/docs/cli',
				destination: '/docs/cloud/cli',
				permanent: false,
			},
			{
				source: '/docs/config',
				destination: '/docs/cloud/config',
				permanent: false,
			},
			{
				source: '/docs/containers',
				destination: '/docs/cloud/containers',
				permanent: false,
			},
			{
				source: '/docs/continuous-delivery',
				destination: '/docs/cloud/continuous-delivery',
				permanent: false,
			},
			{
				source: '/docs/durability',
				destination: '/docs/cloud/durability',
				permanent: false,
			},
			{
				source: '/docs/edge',
				destination: '/docs/cloud/edge',
				permanent: false,
			},
			{
				source: '/docs/environment-variables',
				destination: '/docs/cloud/environment-variables',
				permanent: false,
			},
			{
				source: '/docs/faq',
				destination: '/docs/cloud/faq',
				permanent: false,
			},
			{
				source: '/docs/functions',
				destination: '/docs/cloud/functions',
				permanent: false,
			},
			{
				source: '/docs/hub',
				destination: '/docs/cloud/hub',
				permanent: false,
			},
			{
				source: '/docs/install',
				destination: '/docs/cloud/install',
				permanent: false,
			},
			{
				source: '/docs/limitations',
				destination: '/docs/cloud/limitations',
				permanent: false,
			},
			{
				source: '/docs/local-development',
				destination: '/docs/cloud/local-development',
				permanent: false,
			},
			{
				source: '/docs/networking',
				destination: '/docs/cloud/networking',
				permanent: false,
			},
			{
				source: '/docs/pricing',
				destination: '/docs/cloud/pricing',
				permanent: false,
			},
			{
				source: '/docs/quickstart',
				destination: '/docs/cloud/quickstart',
				permanent: false,
			},
			{
				source: '/docs/self-hosting',
				destination: '/docs/cloud/self-hosting',
				permanent: false,
			},
			{
				source: '/docs/self-hosting/:slug*',
				destination: '/docs/cloud/self-hosting/:slug*',
				permanent: false,
			},
			{
				source: '/docs/solutions/:slug*',
				destination: '/docs/cloud/solutions/:slug*',
				permanent: false,
			},
			{
				source: '/docs/tokens',
				destination: '/docs/cloud/tokens',
				permanent: false,
			},
			{
				source: '/docs/troubleshooting',
				destination: '/docs/cloud/troubleshooting',
				permanent: false,
			},
			{
				source: '/docs/workers',
				destination: '/docs/cloud/workers',
				permanent: false,
			},
			{
				source: '/docs/javascript-runtime',
				destination: '/docs/cloud/actors',
				permanent: false,
			},
			{
				source: '/docs/container-runtime',
				destination: '/docs/cloud/containers',
				permanent: false,
			},

			// Additional redirects for missing pages from CSV
			{
				source: '/docs/general/authentication',
				destination: '/docs/actors/authentication',
				permanent: false,
			},
			{
				source: '/docs/general/authentication/',
				destination: '/docs/actors/authentication',
				permanent: false,
			},
			{
				source: '/docs/general/testing',
				destination: '/docs/actors/testing',
				permanent: false,
			},
			{
				source: '/docs/general/testing/',
				destination: '/docs/actors/testing',
				permanent: false,
			},
			{
				source: '/docs/actors/communicating-with-actors',
				destination: '/docs/actors/communicating-between-actors',
				permanent: false,
			},
			{
				source: '/docs/actors/communicating-with-actors/',
				destination: '/docs/actors/communicating-between-actors',
				permanent: false,
			},
			{
				source: '/actors/communicating-with-actors',
				destination: '/docs/actors/communicating-between-actors',
				permanent: false,
			},
			{
				source: '/actors/communicating-with-actors/',
				destination: '/docs/actors/communicating-between-actors',
				permanent: false,
			},
			{
				source: '/clients/javascript',
				destination: '/docs/clients/javascript',
				permanent: false,
			},
			{
				source: '/clients/javascript/',
				destination: '/docs/clients/javascript',
				permanent: false,
			},

			// Handle talk-to-an-engineer typo in CSV
			{
				source: '/talk-to-an-engineer%20/',
				destination: '/talk-to-an-engineer',
				permanent: false,
			},

			// Handle specific API variants without trailing slash that may not be caught by :slug* pattern
			{
				source: '/docs/api/regions/list',
				destination: '/docs/cloud/api/regions/list',
				permanent: false,
			},
			{
				source: '/docs/api/actors/create',
				destination: '/docs/cloud/api/actors/create',
				permanent: false,
			},
			{
				source: '/docs/api/actors/upgrade-all',
				destination: '/docs/cloud/api/actors/upgrade-all',
				permanent: false,
			},
			{
				source: '/docs/api/actors/upgrade',
				destination: '/docs/cloud/api/actors/upgrade',
				permanent: false,
			},
			{
				source: '/docs/api/actors/destroy',
				destination: '/docs/cloud/api/actors/destroy',
				permanent: false,
			},
			{
				source: '/docs/api/routes/update',
				destination: '/docs/cloud/api/routes/update',
				permanent: false,
			},
		];
	},
	webpack: (config) => {
		const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
		return {
			...config,
			resolve: {
				...config.resolve,
				fallback: {
					"react/jsx-dev-runtime": path.resolve(
						__dirname,
						"../node_modules/react/jsx-dev-runtime.js",
					),
					react: path.resolve(__dirname, "../node_modules/react"),
				},
			},
		};
	},
};

export default async function () {
	return withMDX(nextConfig);
}
