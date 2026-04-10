import type { TemplateContext } from "../../context";

export function generateRunner(context: TemplateContext) {
	// The test runner service now uses the Rust test-envoy binary.
	// The docker-compose template points at the Rust Dockerfile directly.
}
