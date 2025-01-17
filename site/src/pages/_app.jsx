import { MDXProvider } from "@mdx-js/react";
import Head from "next/head";

import { Layout } from "@/components/Layout";
import { Providers } from "@/components/Providers";
import * as mdxComponents from "@/components/mdx";

import "@/styles/tailwind.css";
import "@/styles/fonts.css";
import "focus-visible";

import { config } from "@fortawesome/fontawesome-svg-core";
import "@fortawesome/fontawesome-svg-core/styles.css";
config.autoAddCss = false;

import { TooltipProvider } from "@rivet-gg/components";

import { useNavigation } from "@/hooks/useNavigation";
import { usePathname } from "next/navigation";
import { getSiteUrl } from "../lib/siteUrl";

export default function App({ Component, pageProps }) {
	const siteUrl = getSiteUrl();
	const pathname = usePathname();

	const { navigation, page } = useNavigation();

	let title = pageProps.title ?? Component.title ?? page?.title ?? null;
	title = title ? `${title} - Rivet` : "Rivet";
	const isTopPage = navigation.prefix === pathname;
	const description =
		pageProps.description ??
		Component.description ??
		page?.description ??
		null;
	const tableOfContents =
		(navigation?.tableOfContents?.[pathname] ?? true) &&
		page?.headings?.length > 0
			? page?.headings
			: null;

	return (
		<>
			<Providers>
				<Head>
					<meta
						name="viewport"
						content="width=device-width, initial-scale=1.0"
					/>

					{/* Add common metadata */}
					<meta property="og:image:type" content="image/png" />
					<meta property="og:type" content="website" />
					<meta property="og:url" content="https://rivet.gg/" />

					<meta name="twitter:card" content="summary_large_image" />
					<meta name="twitter:site" content="@rivet_gg" />
					<meta
						name="twitter:image"
						content={`${siteUrl}/promo/og.png`}
					/>
					<meta
						name="twitter:image:alt"
						content="Rivet - Run and scale realtime applications"
					/>
					{/* Add dynamic metadata. Blog `ArticleLayout` provides its own title. */}
					{!pathname.startsWith("/blog/") && (
						<>
							<title>{title}</title>
							{description && (
								<meta
									name="description"
									content={description}
								/>
							)}

							<meta property="og:title" content={title} />
							{description && (
								<meta
									property="og:description"
									content={description}
								/>
							)}
							<meta
								property="og:image"
								content={`${siteUrl}/promo/og.png`}
							/>
							<meta
								property="og:image:alt"
								content="Rivet - Run and scale realtime applications"
							/>

							<meta name="twitter:title" content={title} />
							{description && (
								<meta
									property="twitter:description"
									content={description}
								/>
							)}
						</>
					)}
				</Head>
				<MDXProvider components={mdxComponents}>
					<TooltipProvider>
						<Layout
							navigation={navigation}
							tableOfContents={tableOfContents}
							pathname={pathname}
							prose={Component.prose ?? true}
							inset={Component.inset ?? false}
							fullWidth={Component.fullWidth ?? false}
							isTopPage={isTopPage || Component.isTopPage}
							{...pageProps}
						>
							<Component {...pageProps} />
						</Layout>
					</TooltipProvider>
				</MDXProvider>
			</Providers>
		</>
	);
}
