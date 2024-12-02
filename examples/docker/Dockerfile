FROM alpine:latest
RUN apk add --no-cache shadow
RUN useradd -m -s /bin/sh rivet
USER rivet
CMD while true; do echo "Hello, World!"; sleep 10; done
