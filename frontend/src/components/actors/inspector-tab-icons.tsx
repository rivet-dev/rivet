import {
	faCubesStacked,
	faDatabase,
	faDiagramProject,
	faInbox,
	faLogs,
	faPlug,
	faQuestionSquare,
	faTag,
	faTerminal,
} from "@rivet-gg/icons";

type IconDefinition = typeof faQuestionSquare;

// Registry mapping icon identifiers (sent over the postMessage bridge as
// opaque strings by the iframe) to FontAwesome icon components on the
// dashboard side. Both sides agree on the identifier vocabulary; the actual
// icon visuals can change independently per build.
//
// Unknown identifiers fall back to `faQuestionSquare` so the strip stays
// usable when an iframe advertises a tab whose icon the dashboard doesn't
// yet know about (e.g. older dashboard, newer actor).
const ICON_REGISTRY: Record<string, IconDefinition> = {
	workflow: faDiagramProject,
	database: faDatabase,
	state: faCubesStacked,
	queue: faInbox,
	plug: faPlug,
	terminal: faTerminal,
	tag: faTag,
	logs: faLogs,
};

export function resolveInspectorTabIcon(id: string): IconDefinition {
	return ICON_REGISTRY[id] ?? faQuestionSquare;
}
