import { faExclamationTriangle, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import { useMatch, useParams } from "@tanstack/react-router";
import { AnimatePresence, motion } from "framer-motion";
import { useEffect } from "react";
import { Button, cn } from "@/components";
import { useCloudDataProvider } from "@/components/actors";
import { BYOC_TRIAL_DAYS } from "@/content/byoc";
import { features } from "@/lib/features";
import { ByocContactTrigger } from "./byoc-contact-trigger";

const BANNER_HEIGHT = "2.25rem";

const MS_PER_DAY = 24 * 60 * 60 * 1000;

export function daysLeftInTrial(createdAt: string | undefined) {
	const started = createdAt ? new Date(createdAt).getTime() : Number.NaN;
	if (Number.isNaN(started)) return null;
	const ends = started + BYOC_TRIAL_DAYS * MS_PER_DAY;
	return Math.ceil((ends - Date.now()) / MS_PER_DAY);
}

export function ByocTrialAlert() {
	if (!features.byoc) return null;
	return <ByocTrialAlertGuard />;
}

function ByocTrialAlertGuard() {
	const clusterMatch = useMatch({
		from: "/_context/orgs/$organization/clusters/$cluster",
		shouldThrow: false,
	});

	if (!clusterMatch) return null;

	return <ByocTrialAlertInner />;
}

function ByocTrialAlertInner() {
	const { cluster } = useParams({
		from: "/_context/orgs/$organization/clusters/$cluster",
	});
	const dataProvider = useCloudDataProvider();
	const { data } = useQuery(
		dataProvider.currentOrgClusterQueryOptions({ cluster }),
	);

	const daysLeft = daysLeftInTrial(data?.createdAt);
	const hidden = daysLeft === null;

	useEffect(() => {
		if (hidden) return;
		const root = document.documentElement;
		root.style.setProperty("--billing-banner-height", BANNER_HEIGHT);
		return () => {
			root.style.removeProperty("--billing-banner-height");
		};
	}, [hidden]);

	const ended = daysLeft !== null && daysLeft <= 0;

	return (
		<AnimatePresence>
			{!hidden ? (
				<motion.div
					initial={{ height: 0, opacity: 0 }}
					animate={{ height: BANNER_HEIGHT, opacity: 1 }}
					exit={{ height: 0, opacity: 0 }}
					transition={{ duration: 0.25, ease: "easeOut" }}
					className={cn(
						"overflow-hidden border-b",
						ended
							? "border-destructive/60 bg-destructive/15"
							: "border-warning/60 bg-warning/10",
					)}
				>
					<div className="flex h-9 items-center gap-2 px-3 text-xs">
						<Icon
							icon={faExclamationTriangle}
							className={cn(
								"shrink-0",
								ended ? "text-destructive" : "text-warning",
							)}
						/>
						<p className="text-foreground font-medium">
							{ended
								? "Trial ended for this cluster"
								: "BYOC trial"}
						</p>
						<p className="text-muted-foreground min-w-0 truncate">
							{ended
								? "Contact us for uninterrupted access to this cluster."
								: `${daysLeft} ${daysLeft === 1 ? "day" : "days"} left in your free trial.`}
						</p>
						<ByocContactTrigger>
							{(open) => (
								<Button
									size="sm"
									variant="ghost"
									className="ml-auto h-6 shrink-0 text-xs"
									onClick={open}
								>
									Contact us
								</Button>
							)}
						</ByocContactTrigger>
					</div>
				</motion.div>
			) : null}
		</AnimatePresence>
	);
}
