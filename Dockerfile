FROM alpine

COPY server/target/x86_64-unknown-linux-musl/release/server /

ENV ADDRESS 0.0.0.0:8000
RUN chmod +x /server
ENTRYPOINT ["/server"]
