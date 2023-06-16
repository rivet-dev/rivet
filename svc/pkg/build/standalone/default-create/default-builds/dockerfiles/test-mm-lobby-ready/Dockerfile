FROM alpine:3.14
RUN apk --no-cache add curl
WORKDIR /app
RUN echo 'echo "Env:"; env; echo "resolv.conf"; cat /etc/resolv.conf; READY_URL="$RIVET_MATCHMAKER_API_URL/lobbies/ready"; echo "Sending ready to $READY_URL"; curl --fail --insecure --request POST --header "Content-Type: application/json" --header "Authorization: Bearer $RIVET_TOKEN" --data "{}" "$READY_URL"; echo "Success"; echo "Waiting indefinitely"; tail -f /dev/null;' > ./run.sh
RUN chmod +x ./run.sh
CMD ["sh", "-ecx", "/app/run.sh"]

