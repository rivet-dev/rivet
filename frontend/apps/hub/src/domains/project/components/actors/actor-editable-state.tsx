import {
	Badge,
	Button,
	DocsSheet,
	LiveBadge,
	WithTooltip,
} from "@rivet-gg/components";
import {
	type CodeMirrorRef,
	EditorView,
	JsonCode,
} from "@rivet-gg/components/code-mirror";
import { Icon, faBooks, faRotateLeft, faSave } from "@rivet-gg/icons";
import { AnimatePresence, motion } from "framer-motion";
import { useMemo, useRef, useState } from "react";
import { ActorStateChangeIndicator } from "./actor-state-change-indicator";
import type { ContainerState } from "./worker/actor-worker-container";
import { useActorWorker } from "./worker/actor-worker-context";

const isValidJson = (json: string | null): json is string => {
	if (!json) return false;
	try {
		JSON.parse(json);
		return true;
	} catch {
		return false;
	}
};

interface ActorEditableStateProps {
	state: ContainerState["state"];
}

export function ActorEditableState({ state }: ActorEditableStateProps) {
	const container = useActorWorker();
	const [isEditing, setIsEditing] = useState(false);
	const [value, setValue] = useState<string | null>(null);

	const ref = useRef<CodeMirrorRef>(null);

	const formatted = useMemo(() => {
		return JSON.stringify(state.value || "{}", null, 2);
	}, [state.value]);

	const isValid = isValidJson(value)
		? JSON.parse(value)
		: false;

	return (
		<>
			<div className="flex justify-between items-center border-b gap-1 p-2">
				<div className="flex items-center justify-start gap-1">
					<LiveBadge />

					<ActorStateChangeIndicator state={state.value} />
				</div>
				<div className="flex gap-2">
					<AnimatePresence>
						{isEditing ? (
							<WithTooltip
								trigger={
									<Badge variant="outline" asChild>
										<motion.div
											initial={{ opacity: 0 }}
											animate={{ opacity: 1 }}
										>
											Modified
										</motion.div>
									</Badge>
								}
								content="State has been modified and not saved."
							/>
						) : null}
					</AnimatePresence>
					<WithTooltip
						content="Save state"
						trigger={
							<Button
								size="icon-sm"
								variant="outline"
								disabled={!isValid || !isEditing}
								onClick={() => {
									container.setState(value || "");
									setIsEditing(false);
									setValue(null);
								}}
							>
								<Icon icon={faSave} />
							</Button>
						}
					/>
					<WithTooltip
						content="Restore original state"
						trigger={
							<Button
								size="icon-sm"
								variant="outline"
								disabled={!isEditing}
								onClick={() => {
									setValue(null);
									setIsEditing(false);
								}}
							>
								<Icon icon={faRotateLeft} />
							</Button>
						}
					/>
					<DocsSheet title="State" path="docs/state">
						<Button
							variant="outline"
							size="sm"
							startIcon={<Icon icon={faBooks} />}
						>
							Documentation
						</Button>
					</DocsSheet>
				</div>
			</div>
			<div className="flex flex-1 min-h-0 w-full">
				<JsonCode
					ref={ref}
					value={value || formatted}
					extensions={[EditorView.lineWrapping]}
					className="flex-1 flex w-full min-h-0 [&>div]:w-full"
					onChange={(value) => {
						setValue(value);
						setIsEditing(true);
					}}
				/>
			</div>
		</>
	);
}
