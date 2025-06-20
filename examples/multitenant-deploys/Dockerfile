FROM node:18-alpine

WORKDIR /app

# Install rivet CLI
RUN apk add --no-cache curl unzip
RUN curl -fsSL https://releases.rivet.gg/rivet/latest/install.sh | sh

# Copy package files and install dependencies
COPY package.json yarn.lock ./
RUN yarn install --frozen-lockfile

# Copy application code
COPY . .

# Build the application
RUN yarn build

# Expose the port the app will run on
EXPOSE 3000

# Create a non-root user
RUN addgroup -S appgroup && adduser -S appuser -G appgroup
USER appuser

# Start the application
CMD ["node", "dist/index.js"]