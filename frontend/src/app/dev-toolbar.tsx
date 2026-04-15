import { useAuth, useOrganization, useUser } from "@clerk/clerk-react";
import * as Sentry from "@sentry/react";
import {
	formatForDisplay,
	type Hotkey,
	useHotkeySequence,
	useKeyHold,
} from "@tanstack/react-hotkeys";
import { useQueryClient } from "@tanstack/react-query";
import { usePostHog } from "posthog-js/react";
import { useEffect, useState } from "react";
import { Kbd, ls } from "@/components";

export const DevToolbar = () => {
	if (__APP_TYPE__ !== "cloud") return null;
	if (
		ls.get(
			"__I_SOLELY_SWORE_TO_ONLY_ENABLE_DEV_TOOLBAR_FOR_DEBUGGING_PURPOSES_AND_WILL_NOT_USE_IT_FOR_ANY_MALICIOUS_ACTIVITIES__",
		) !== true
	) {
		return null;
	}

	return <Content />;
};

const debugSequence: Hotkey = "Mod+Shift+G";
const posthogDebugSequence: Hotkey[] = [debugSequence, "R"];
const posthogStartRecordingSequence: Hotkey[] = [...posthogDebugSequence, "S"];
const posthogStopRecordingSequence: Hotkey[] = [...posthogDebugSequence, "T"];
const posthogLoadToolbarSequence: Hotkey[] = [...posthogDebugSequence, "L"];
const clearCacheSequence: Hotkey[] = [debugSequence, "C"];
const reportIssueSequence: Hotkey[] = [debugSequence, "I"];

const Content = () => {
	const { userId, actor } = useAuth();
	const { user } = useUser();
	const { organization: org } = useOrganization();

	const [, setState] = useState({}); // used just to trigger re-render every second

	useEffect(() => {
		const interval = setInterval(() => {
			setState({});
		}, 1000);
		return () => clearInterval(interval);
	}, []);

	const posthog = usePostHog();
	const queryClient = useQueryClient();

	// recording start
	useHotkeySequence(posthogStartRecordingSequence, () => {
		posthog.startSessionRecording();
	});
	// recording stop
	useHotkeySequence(posthogStopRecordingSequence, () => {
		posthog.stopSessionRecording();
	});
	// load toolbar
	useHotkeySequence(posthogLoadToolbarSequence, () => {
		posthog.toolbar.loadToolbar();
	});
	// clear cache
	useHotkeySequence(clearCacheSequence, () => {
		queryClient.clear();
	});
	// report issue
	useHotkeySequence(reportIssueSequence, () => {
		Sentry.showReportDialog();
	});

	return (
		<div className="fixed bottom-0 inset-x-0 bg-background-main px-2 border-t border-border-main text-xs z-50 flex gap-0.5 text-muted-foreground overflow-auto">
			<div className="flex min-w-0 flex-shrink-0 items-center h-8 [&>div]:h-10 [&>div]:flex [&>div]:items-center [&>div]:min-w-0 [&>div]:flex-shrink-0 [&>div]:gap-0.5 [&_button[disabled]]:cursor-not-allowed [&_button[disabled]]:opacity-80 [&_button[disabled]]:no-underline [&_button]:underline ">
				<div>
					{__APP_TYPE__}{" "}
					<span className="font-mon">{__APP_BUILD_ID__}</span>
				</div>
				<Sep />
				<div>
					{actor ? (
						<span className="bg-warning/20 p-0.5 rounded-md">
							{actor?.sub || "Unknown"} is signed in as{" "}
							{userId || "Unknown"}
						</span>
					) : null}
					<span>
						{user?.primaryEmailAddress?.emailAddress || "Unknown"}
					</span>
					<span>
						{user?.id || "Unknown"} {org?.id || "Unknown"}
					</span>
				</div>

				<Sep />
				<div>
					<span>rec {posthog.sessionRecording?.status}</span>
					<div>
						<button
							type="button"
							disabled={posthog.sessionRecording?.started}
							onClick={() => {
								posthog.startSessionRecording();
							}}
						>
							{posthog.sessionRecording?.started
								? "recording..."
								: "start"}
						</button>{" "}
						<ShortcutBadge
							hotkey={posthogStartRecordingSequence.join(" ")}
						/>
					</div>{" "}
					<button
						type="button"
						disabled={!posthog.sessionRecording?.started}
						onClick={() => {
							posthog.stopSessionRecording();
						}}
					>
						stop
					</button>{" "}
					<ShortcutBadge
						hotkey={posthogStopRecordingSequence.join(" ")}
					/>{" "}
					⋅{" "}
					<button
						type="button"
						onClick={() => {
							posthog.toolbar.loadToolbar();
						}}
					>
						load toolbar
					</button>{" "}
					<ShortcutBadge
						hotkey={posthogLoadToolbarSequence.join(" ")}
					/>
				</div>

				<Sep />
				<div>
					<button
						type="button"
						onClick={() => {
							Sentry.showReportDialog();
						}}
					>
						report issue
					</button>{" "}
					<ShortcutBadge hotkey={reportIssueSequence.join(" ")} />
				</div>
				<Sep />
				<div>
					<button
						type="button"
						className="underline"
						onClick={() => {
							queryClient.clear();
						}}
					>
						clear cache
					</button>
					<ShortcutBadge hotkey={clearCacheSequence.join(" ")} />
				</div>
			</div>
		</div>
	);
};

function ShortcutBadge({ hotkey }: { hotkey: Hotkey | (string & {}) }) {
	const isShiftHeld = useKeyHold("Shift");
	if (isShiftHeld) {
		return <Kbd className="shortcut-badge">{formatForDisplay(hotkey)}</Kbd>;
	}
	return null;
}

const Sep = () => <span className=" text-muted-foreground">⋅</span>;
