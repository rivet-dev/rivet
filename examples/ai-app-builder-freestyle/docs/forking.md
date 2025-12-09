# Forking Guide

This documentation is for developers who want to work with the Adorable codebase. It's a good starting place to experiment with building your own AI-powered app builder.

For additional context on building app builders with AI, see the [Freestyle guide on Building an App Builder](https://docs.freestyle.sh/guides/app-builder).

## This is where the prompt goes

The system prompt is located in `src/lib/system.ts` and exported as `SYSTEM_MESSAGE`.

**Tips for updating the system prompt:**

- Keep instructions clear and specific
- Test changes by creating a new app to see how the AI behaves
- The prompt defines the AI's personality and workflow preferences
- Consider adding domain-specific guidance for your use case

## This is where the tools go

Tools are located in the `src/tools/` directory. Each tool is a Mastra tool created with `createTool()`.

**Existing tools:**

- `todo-tool.ts` - Manages todo lists for the AI agent to track tasks
- `morph-tool.ts` - File editing tool that uses Morph API for code modifications

**Adding new tools:**

1. Create a new file in `src/tools/`
2. Export the tool using Mastra's `createTool()` function
3. Import and add it to the `tools` object in `src/mastra/agents/builder.ts`

The tools are automatically available to the AI agent through the chat API in `src/app/api/chat/route.ts`.

Add context, web search, databases or any other tools to supercharge the AI's capabilities and make it more useful for your specific app-building needs.

## This is where the model goes

The AI model configuration is in `src/lib/model.ts`.

By default, we use Claude 4 Sonnet, but GPT-5 and Claude 4 Opus also work well.

## This is where the UI goes

**Important UI files:**

- `src/app/page.tsx` - Main landing page with prompt input and examples
- `src/app/app/[id]/page.tsx` - Individual app chat interface
- `src/components/chat.tsx` - Main chat component for AI interactions
- `src/components/ui/` - Reusable UI components (buttons, inputs, etc.)
- `src/components/user-apps.tsx` - User's app list display
- `src/components/webview.tsx` - App preview iframe component

## Where the database stuff goes

**Database configuration:**

- `src/db/schema.ts` - Database schema definitions and db connection
- Server actions in `src/actions/` handle database operations
- Use Drizzle ORM for type-safe database queries
- Run `npx drizzle-kit push` to apply schema changes

## Database Schema

The database schema is defined in `src/db/schema.ts` and includes:

- `appsTable` - Application metadata
- `appUsers` - User permissions for apps
- `messagesTable` - Chat message history
- `appDeployments` - Deployment tracking

## Environment Variables

Required environment variables:

- `DATABASE_URL` - PostgreSQL connection string
- `REDIS_URL` - Redis connection string
- `ANTHROPIC_API_KEY` - Claude API key
- `FREESTYLE_API_KEY` - Freestyle platform key
- Stack Auth variables (see README.md)
