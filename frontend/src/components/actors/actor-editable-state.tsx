import { faRotateLeft, faSave, Icon } from "@rivet-gg/icons";
import { AnimatePresence, motion } from "framer-motion";
import { useMemo, useRef, useState } from "react";
import {
	Badge,
	Button,
	LiveBadge,
	PauseBadge,
	WithTooltip,
} from "@/components";
import {
	type CodeMirrorRef,
	EditorView,
	JsonCode,
} from "@/components/code-mirror";
import { ActorStateChangeIndicator } from "./actor-state-change-indicator";
import {
	type ActorId,
	useActorStatePatchMutation,
	useActorStateStream,
} from "./queries";

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
	actorId: ActorId;
	state: unknown;
}

export function ActorEditableState({
	state,
	actorId,
}: ActorEditableStateProps) {
	const [isEditing, setIsEditing] = useState(false);
	const [value, setValue] = useState<string | null>(null);

	const ref = useRef<CodeMirrorRef>(null);

	const formatted = useMemo(() => {
		return JSON.stringify(state, null, 2);
	}, [state]);

	const isValid = isValidJson(value) ? JSON.parse(value) : false;

	const { mutate, isPending } = useActorStatePatchMutation(actorId);

	// useActorStateStream(actorId);

	return (
		<>
			<div className="flex justify-between items-center border-b gap-1 p-2 h-[45px]">
				<div className="flex items-center justify-start gap-1">
					{isEditing ? <PauseBadge /> : <LiveBadge />}

					<ActorStateChangeIndicator state={state} />
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
								isLoading={isPending}
								disabled={!isValid || !isEditing}
								onClick={() => {
									mutate(JSON.parse(value || ""));
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
