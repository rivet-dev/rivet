# User and AI Generated Rivet Actors Freestyle Deployer

Shows how to deploy user or AI-generated Rivet Actor code using a sandboxed namespace and Freestyle

[View Full Documentation](https://rivet.dev/docs/actors/ai-and-user-generated-actors)

## Features

- **Sandboxed Rivet namespace**: Create isolated environments for testing and deploying actors
- **Deploy user or AI generated code**: Deploy custom actor and frontend code directly to Freestyle
- **Automatic configuration**: Configure Rivet & Freestyle together automatically without manual setup

## How It Works

The logic lives in `src/backend/`:

1. **Receives user or AI-generated code**: The backend receives custom actor code (`registry.ts`) and frontend code (`App.tsx`)
2. **Creates a sandboxed Rivet namespace**: Either using Rivet Cloud or Rivet self-hosted API (see [`deploy-with-rivet-cloud.ts`](src/backend/deploy-with-rivet-cloud.ts) and [`deploy-with-rivet-self-hosted.ts`](src/backend/deploy-with-rivet-self-hosted.ts))
3. **Deploys actor and frontend to Freestyle**: Builds the project and deploys to Freestyle (see [`src/backend/utils.ts`](src/backend/utils.ts))
4. **Configures Rivet to connect to Freestyle**: Sets up the Rivet runner to point to the Rivet deployment for running the actors

## Getting Started

### Prerequisites

- Node.js 18+ and pnpm
- A [Freestyle](https://freestyle.sh) account and API key
- Either:
  - [Rivet Cloud](https://dashboard.rivet.dev/) account with API token, or
  - [Self-hosted Rivet instance](https://www.rivet.dev/docs/self-hosting/) with endpoint and token

### Installation

```bash
pnpm install
```

### Running the Application

Start both the frontend and backend servers:

```bash
pnpm dev
```

Open http://localhost:5173 in your browser.

## Usage

### Rivet Cloud

1. **Get Your Cloud API Token**:
   - Go to or create your project on [Rivet Cloud](https://dashboard.rivet.dev/)
   - Click on "Tokens" in the sidebar
   - Under "Cloud API Tokens" click "Create Token" and copy the token
2. **Edit Code**: Modify the `registry.ts` and `App.tsx` code in the editors
3. **Configure Deploy Config**: Fill in all required fields:
   - Rivet Cloud API token (from step 1)
   - Freestyle domain (e.g., myapp.style.dev)
   - Freestyle API key
4. **Deploy**: Click "Deploy to Freestyle" and watch the deployment logs

### Rivet Self-Hosted

1. **Edit Code**: Modify the `registry.ts` and `App.tsx` code in the editors
2. **Configure Deploy Config**: Fill in all required fields:
   - Rivet endpoint (your self-hosted instance URL)
   - Rivet API token
   - Freestyle domain (e.g., myapp.style.dev)
   - Freestyle API key
3. **Deploy**: Click "Deploy to Freestyle" and watch the deployment logs

## Project Structure

```
src/
	backend/  # Backend used to deploy your sandboxed Rivet backend code
	frontend/ # Frontend for the deploy UI you'll be using
template/  # The Rivet template code to deploy with Rivet
	src/
		backend/ # Actor code to be deployed
		frontend/ # Frontend to be deployed
tests/ # Vitest tests
```

## License

MIT
