import { useEffect, useRef } from "react";
import { useIframeTabBridge } from "./iframe-tab-bridge";

interface IframeTabContentProps {
	/** Tab identifier, used as the path segment in /ui/tabs/<tab>/ */
	tab: string;
	actorId: string;
}

/**
 * Renders an inspector tab as a sandboxed iframe pointed at the tab bundle
 * served from the actor's inspector endpoint.
 *
 * Visibility is managed by the parent TabsContent's `forceMount` + Radix's
 * `hidden` attribute, which hides inactive tabs without unmounting them —
 * so the iframe stays loaded across tab switches.
 */
export function IframeTabContent({ tab, actorId }: IframeTabContentProps) {
	const iframeRef = useRef<HTMLIFrameElement>(null);
	const { registerIframe, unregisterIframe } = useIframeTabBridge();

	useEffect(() => {
		const iframe = iframeRef.current;
		if (!iframe) return;
		registerIframe(tab, iframe);
		return () => unregisterIframe(tab);
	}, [tab, registerIframe, unregisterIframe]);

	const shellOrigin = encodeURIComponent(window.location.origin);
	const src = `/ui/tabs/${tab}/?actorId=${encodeURIComponent(actorId)}&shellOrigin=${shellOrigin}`;

	return (
		<iframe
			ref={iframeRef}
			src={src}
			sandbox="allow-scripts allow-same-origin"
			style={{ border: "none", width: "100%", height: "100%", flex: 1 }}
			className="flex-1 min-h-0"
			title={`Inspector ${tab} tab`}
		/>
	);
}
