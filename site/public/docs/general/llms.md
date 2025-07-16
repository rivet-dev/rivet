# LLMs & AI Integration

Rivet provides optimized documentation formats specifically designed for Large Language Models (LLMs) and AI integration tools.

## Available Formats

### LLMs.txt (Condensed)
A condensed version of the documentation perfect for quick reference and context-aware AI assistance.

**Access:** [/llms.txt](/llms.txt)

This format includes:
- Key concepts and features
- Essential getting started information
- Summaries of main functionality
- Optimized for token efficiency

### LLMs-full.txt (Complete)
The complete documentation in a single file, ideal for comprehensive AI assistance and in-depth analysis.

**Access:** [/llms-full.txt](/llms-full.txt)

This format includes:
- Complete documentation content
- All examples and detailed explanations
- Full API references and guides
- Suitable for complex queries and comprehensive understanding

## Individual Page Access

Each documentation page is also available as clean markdown by appending `.md` to any documentation URL path.

### Examples

- **This page as markdown:** [/docs/general/llms.md](/docs/general/llms.md)
- **Actors overview:** [/docs/actors.md](/docs/actors.md)
- **State management:** [/docs/actors/state.md](/docs/actors/state.md)
- **React quickstart:** [/docs/actors/quickstart/react.md](/docs/actors/quickstart/react.md)

### URL Pattern

```
Original URL: https://rivet.gg/docs/[path]
Markdown URL: https://rivet.gg/docs/[path].md
```

## Integration Examples

### ChatGPT/Claude Integration

Use the dropdown on any documentation page to:
- Copy page content directly to clipboard
- Open the page content in ChatGPT or Claude
- View the page as raw markdown

### Custom AI Tools

Fetch documentation programmatically:

```javascript
// Get condensed documentation
const condensed = await fetch('https://rivet.gg/llms.txt').then(r => r.text());

// Get complete documentation
const complete = await fetch('https://rivet.gg/llms-full.txt').then(r => r.text());

// Get specific page as markdown
const actorsDoc = await fetch('https://rivet.gg/docs/actors.md').then(r => r.text());
```

### Embeddings and Vector Databases

The individual `.md` files are perfect for creating embeddings:

```python
import requests

# Fetch a specific page
response = requests.get('https://rivet.gg/docs/actors/state.md')
markdown_content = response.text

# Use with your embedding model
embeddings = embed_model.encode(markdown_content)
```

## Content Processing

All generated files are processed to:
- Remove MDX-specific syntax
- Strip imports and exports
- Start from the first H1 heading
- Maintain clean markdown formatting
- Exclude cloud-specific documentation (focused on open-source)

This ensures maximum compatibility with AI tools and LLMs while maintaining readability and structure.