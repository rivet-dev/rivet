FROM alpine:3.14

RUN apk --no-cache add curl
WORKDIR /app
COPY ./run.sh /app/run.sh
RUN chmod +x ./run.sh
CMD ["sh", "-ecx", "/app/run.sh"]

