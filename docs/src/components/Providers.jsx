'use client';
import { Suspense, useEffect } from 'react';
import { usePathname, useSearchParams } from 'next/navigation';
import posthog from 'posthog-js';
import { PostHogProvider, usePostHog } from 'posthog-js/react';
import { useMobileNavigationStore } from '@/components/MobileNavigation';

// Check that PostHog is client-side
if (typeof window !== 'undefined') {
  posthog.init('phc_6kfTNEAVw7rn1LA51cO3D69FefbKupSWFaM7OUgEpEo', {
    api_host: 'https://ph.rivet.gg',
    // Enable debug mode in development
    loaded: posthog => {
      if (process.env.NODE_ENV === 'development') posthog.debug();
    }
  });
}

function PageViewTracker() {
  let pathname = usePathname();
  let posthog = usePostHog();
  const searchParams = useSearchParams();

  useEffect(() => {
    // Track page views
    let url = window.origin + pathname;
    if (searchParams.toString()) {
      url = url + `?${searchParams.toString()}`;
    }
    posthog.capture('$pageview', {
      $current_url: url
    });
  }, [pathname, searchParams, posthog]);
}

function RouteChangeObserver() {
  let pathname = usePathname();
  useEffect(() => {
    useMobileNavigationStore.getState().close();
  }, [pathname]);
}

export function Providers({ children }) {
  return (
    <PostHogProvider client={posthog}>
      {children}
      <Suspense fallback={null}>
        <PageViewTracker />
      </Suspense>
      <RouteChangeObserver />
    </PostHogProvider>
  );
}
