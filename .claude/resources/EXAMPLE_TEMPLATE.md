# Example README Template

All example READMEs in `/examples/` should follow this structure with exact section headings:

```markdown
<header with title and description>

## Getting Started

[Setup instructions - how to install dependencies and run the example]

## Features

[3-5 features that highlight what this example demonstrates, focusing on RivetKit concepts:]

- Documents dashboard with pagination, drafts, groups, auto-revalidation
- Collaborative whiteboard app with a fully-featured share menu
- Authentication compatible with GitHub, Google, Auth0, and more
- Document permissions can be scoped to users, groups, and the public

## Prerequisites

[Only include this section if there are non-obvious prerequisites like API keys or external services.
Do not include obvious requirements like "Node.js" or "pnpm". For example:]

- OpenAI API Key
- PostgreSQL database

## Implementation

[Explain the key technical concepts, architecture, or implementation details. Always include GitHub
source code links to key files. For example:]

This example works by packaging your application code and uploading it to Freestyle.

See the implementation in [`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/example-name/src/backend/registry.ts).

## Resources

[Link to relevant RivetKit documentation that helps users understand the concepts demonstrated:]

Read more about [actions](/docs/actors/actions) and [state](/docs/actors/state).

## License

[License info - typically MIT]
```

## Section Heading Convention

Use these exact section headings (case-sensitive):
- `## Getting Started` - Setup instructions
- `## Features` - Feature list
- `## Prerequisites` - Only if needed for non-obvious dependencies
- `## Implementation` - Technical details (NOT "How it Works")
- `## Resources` - Links to relevant documentation
- `## License` - License info

## Guidelines

- **Features**: Focus on what RivetKit concepts the example demonstrates, not just what the app does. Highlight patterns like actor communication, state management, WebSocket handling, etc.
- **Prerequisites**: Only include if the example requires non-obvious dependencies (API keys, external services). Skip obvious tooling like Node.js or pnpm.
- **Implementation**: Always required. Explain the technical details and include GitHub source code links to key files.
- **Resources**: Always include links to relevant RivetKit documentation that relates to the concepts in the example.
- **Source Code Links**: Use GitHub links in the format: `https://github.com/rivet-dev/rivet/tree/main/examples/{example-name}/{path}`
  - Example: `https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent/src/backend/actors/agent.ts`
- **Formatting**: Do not use em dashes (—). Use hyphens (-) or rephrase sentences instead.
- Preserve existing header content (title, description) and license section.
