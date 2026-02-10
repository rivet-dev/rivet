# Friction Log

## Initial Misunderstanding of Requirements

- Initially set up a separate backend server with Hono and proxy configuration
- Requirements actually specified "file-system driver (standalone mode, no external server)"
- This means RivetKit should run in the same process as Vite, not as a separate server
- Had to reconsider the architecture and remove the server/proxy approach

## Solution

- Created a custom Vite plugin that handles `/api/rivet` routes directly in the Vite dev server
- This allows RivetKit to run in standalone mode using the file-system driver without needing a separate server process
- The registry.serve() is called within the Vite plugin's configureServer hook
- All RivetKit API calls are handled by middleware that converts between Node.js HTTP and Web API formats
