FROM denoland/deno:2.2.2 as builder
WORKDIR /app
COPY . .

# Create non-root user
RUN adduser --disabled-password --gecos "" rivet
USER rivet

RUN deno cache src/main.ts

CMD ["deno", "run", "-A", "src/main.ts"]

