FROM alpine:3.14

RUN apk --no-cache add curl netcat-openbsd bind-tools
WORKDIR /app
COPY ./run.sh /app/run.sh
RUN chmod +x ./run.sh
RUN adduser -D app
USER app
CMD ["sh", "-ecx", "/app/run.sh"]

