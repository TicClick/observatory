FROM docker.io/rust:alpine3.23 as builder

RUN apk add --no-cache pkgconfig openssl-dev openssl-libs-static musl-dev libcrypto3

WORKDIR /build

COPY src ./src
COPY Cargo.lock Cargo.toml ./

RUN cargo build --release

FROM alpine:3.23.2

WORKDIR /app
COPY --from=builder /build/target/release/observatory .

ENTRYPOINT [ "/app/observatory" ]
