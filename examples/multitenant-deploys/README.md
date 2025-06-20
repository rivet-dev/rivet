# Multitenant Deploys for Rivet

A simple Node.js service for handling multi-tenant deployments with Rivet.

## Features

- Accepts source code uploads via a multipart POST request
- Validates the presence of a Dockerfile
- Deploys the code to Rivet using `rivet publish`
- Sets up a custom domain route for the application

## Getting Started

### Prerequisites

- Node.js
- [Rivet CLI](https://rivet.gg/docs/install)
- Rivet cloud token ([instructions on how to generate](https://rivet.gg/docs/tokens#cloud-token))
- Rivet project ID
	- For example if your project is at `https://hub.rivet.gg/projects/foobar`, the ID is `foobar`
- Rivet environment ID
	- For example if your environment is at `https://hub.rivet.gg/projects/foobar/environments/prod`, the ID is `prod`

### Environment Variables

You'll need to set the following environment variables:

```bash
RIVET_CLOUD_TOKEN=your_rivet_cloud_token
RIVET_PROJECT=your_project_id
RIVET_ENVIRONMENT=your_environment_name
PORT=3000 # Optional, defaults to 3000
```

You can do this by using [`export`](https://askubuntu.com/a/58828) or [dotenv](https://www.npmjs.com/package/dotenv).

### Developing

```bash
yarn install
yarn dev
```

You can now use `POST http://locahlost:3000/deploy/my-app-id`. Read more about example usage below.

### Testing

```bash
yarn test
```

## API Usage

`POST /deploy/:appId`

**Request:**
- URL Path Parameter:
  - `appId`: Unique identifier for the application (3-30 characters, lowercase alphanumeric with hyphens)
- Multipart form data containing:
  - `Dockerfile`: A valid Dockerfile for the application (required)
  - Additional files for the application

**Response:**
```json
{
  "success": true,
  "appId": "your-app-id",
  "endpoint": "https://your-app-id.example.com",
  "buildOutput": "..." // Output logs from build command
}
```

## Example Usage

```javascript
const appId = "my-app-id";

// Form data that includes project files
const formData = new FormData();

const serverContent = `
const http = require("http");
const server = http.createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "text/plain" });
  res.end("Hello from " + process.env.MY_ENV_VAR);
});
server.listen(8080);
`;
const serverBlob = new Blob([serverContent], {
  type: "application/octet-stream"
});
formData.append("server.js", serverBlob, "server.js");

const dockerfileContent = `
FROM node:22-alpine
WORKDIR /app
COPY . .

# Set env var from build arg
ARG MY_ENV_VAR
ENV MY_ENV_VAR=$MY_ENV_VAR

# Create a non-root user
RUN addgroup -S rivetgroup && adduser -S rivet -G rivetgroup
USER rivet

CMD ["node", "server.js"]
`;
const dockerfileBlob = new Blob([dockerfileContent], {
  type: "application/octet-stream"
});
formData.append("Dockerfile", dockerfileBlob, "Dockerfile");

// Run the deploy
const response = fetch(`http://localhost:3000/deploy/${appId}`, {
  method: "POST",
  body: formData
});
if (response.ok) {
  const { endpoint } = await response.json();
}
```
