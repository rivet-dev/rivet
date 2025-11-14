import { Providers } from "@/components/Providers";
import { GoogleAnalytics } from "@next/third-parties/google";
import { Toaster, TooltipProvider } from "@rivet-gg/components";
import type { Metadata } from "next";
import "@fortawesome/fontawesome-svg-core/styles.css";
import Script from "next/script";

let metadataBase: URL | null = null;
if (process.env.METADATA_BASE)
	metadataBase = new URL(process.env.METADATA_BASE);
else if (process.env.CF_PAGES_URL)
	metadataBase = new URL(process.env.CF_PAGES_URL);

export const metadata: Metadata = {
	metadataBase,
	title: "Rivet - Build and scale stateful workloads",
	description:
		"Rivet is a library for long-lived processes with durable state, realtime, and scalability. Easily self-hostable and works with your infrastructure.",
	twitter: {
		site: "@rivet_dev",
		card: "summary_large_image",
	},
	openGraph: {
		type: "website",
		locale: "en_US",
		url: "https://www.rivet.dev",
		siteName: "Rivet",
		images: [
			{
				url: "https://www.rivet.dev/promo/og.png",
				width: 1200,
				height: 630,
				alt: "Rivet",
			},
		],
	},
};

export default function Layout({ children }) {
	return (
		<html lang="en" className="dark">
			<head>
				<GoogleAnalytics gaId="G-GHX1328ZFD" />
				<Script
					src="https://analytics.ahrefs.com/analytics.js"
					data-key="9b9IZaKPAs/KhRrWrBjsrw"
					strategy="beforeInteractive"
				/>
				
				{/* Fonts for landing page */}
				<link rel="preconnect" href="https://fonts.googleapis.com" />
				<link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
				<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&display=swap" rel="stylesheet" />
				<link href="https://api.fontshare.com/v2/css?f[]=satoshi@700,900&display=swap" rel="stylesheet" />
				<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400&display=swap" rel="stylesheet" />

				<link
					rel="apple-touch-icon"
					sizes="180x180"
					href="/icons/apple-touch-icon.png?20240925"
				/>
				<link
					rel="icon"
					type="image/png"
					sizes="32x32"
					href="/icons/favicon-32x32.png?20240925"
				/>
				<link
					rel="icon"
					type="image/png"
					sizes="16x16"
					href="/icons/favicon-16x16.png?20240925"
				/>
				<link rel="manifest" href="/icons/site.webmanifest?20240925" />
				<link
					rel="mask-icon"
					href="/icons/safari-pinned-tab.svg?20240925"
					color="#5bbad5"
				/>
				<meta name="msapplication-TileColor" content="#0c0a09" />
				<meta name="theme-color" content="#0c0a09" />

				<meta
					name="viewport"
					content="width=device-width, initial-scale=1.0"
				/>
			</head>
			<body className="dark">
				<TooltipProvider>
					<Providers>{children}</Providers>
				</TooltipProvider>
				<Toaster />
			</body>
		</html>
	);
}
