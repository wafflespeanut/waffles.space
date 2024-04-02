FROM rust:alpine
RUN apk add --update musl-dev openssl-dev openssl-libs-static g++
COPY . /home/rust/src
WORKDIR /home/rust/src
ENV RUSTFLAGS="-C link-args=-Wl,-Bstatic -C link-args=-lc"
RUN cargo build --release

FROM alpine

RUN apk --update upgrade && \
    apk add --update inotify-tools && \
    rm -rf /var/cache/apk/*

COPY --from=0 /home/rust/src/target/release/server /server

ENV ADDRESS 0.0.0.0:8000
RUN chmod +x /server
ENTRYPOINT ["/server"]
