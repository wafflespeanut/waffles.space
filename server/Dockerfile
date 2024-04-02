FROM rust:alpine
RUN apk add --update musl-dev openssl-dev
COPY . /home/rust/src
WORKDIR /home/rust/src
RUN cargo build --release

FROM alpine

RUN apk --update upgrade && \
    apk add --update inotify-tools && \
    rm -rf /var/cache/apk/*

COPY --from=0 /home/rust/src/target/x86_64-unknown-linux-musl/release/server /server

ENV ADDRESS 0.0.0.0:8000
RUN chmod +x /server
ENTRYPOINT ["/server"]
