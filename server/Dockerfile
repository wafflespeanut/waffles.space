FROM alpine

RUN apk --update upgrade && \
    apk add --update inotify-tools && \
    rm -rf /var/cache/apk/*

COPY target/x86_64-unknown-linux-musl/release/server /

ENV ADDRESS 0.0.0.0:8000
RUN chmod +x /server
ENTRYPOINT ["/server"]
