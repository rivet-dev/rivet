import { GoogleAnalytics } from '@next/third-parties/google';
import { Metadata } from 'next';

import { Toaster, TooltipProvider } from '@rivet-gg/components';
import '@fortawesome/fontawesome-svg-core/styles.css';

let metadataBase: URL | null = null;
if (process.env.METADATA_BASE) metadataBase = new URL(process.env.METADATA_BASE);
else if (process.env.CF_PAGES_URL) metadataBase = new URL(process.env.CF_PAGES_URL);

export const metadata: Metadata = {
  metadataBase,
  title: 'Rivet - Open-Source Multiplayer Tooling',
  description: 'A unified platform to manage your game servers & backend.',
  twitter: {
    site: '@rivetgg',
    card: 'summary_large_image'
  },
  openGraph: {
    type: 'website',
    locale: 'en_US',
    url: 'https://rivet.gg',
    siteName: 'Rivet',
    images: [
      {
        url: 'https://rivet.gg/promo/og.png',
        width: 1200,
        height: 630,
        alt: 'Rivet'
      }
    ]
  }
};

export default function Layout({ children }) {
  return (
    <html lang='en' className='dark'>
      <head>
        <GoogleAnalytics gaId='G-GHX1328ZFD' />

        <link rel='apple-touch-icon' sizes='180x180' href='/icons/apple-touch-icon.png?20240925' />
        <link rel='icon' type='image/png' sizes='32x32' href='/icons/favicon-32x32.png?20240925' />
        <link rel='icon' type='image/png' sizes='16x16' href='/icons/favicon-16x16.png?20240925' />
        <link rel='manifest' href='/icons/site.webmanifest?20240925' />
        <link rel='mask-icon' href='/icons/safari-pinned-tab.svg?20240925' color='#5bbad5' />
        <meta name='msapplication-TileColor' content='#0c0a09' />
        <meta name='theme-color' content='#0c0a09' />

        <meta name='viewport' content='width=device-width, initial-scale=1.0' />
      </head>
      <body className='dark'>
        <TooltipProvider>{children}</TooltipProvider>
        <Toaster />
      </body>
    </html>
  );
}
