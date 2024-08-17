FROM rust:alpine as Builder

WORKDIR /workspace

COPY ./ /workspace

RUN apk update && apk add git build-base && cargo build --release


FROM alpine:3

COPY --from=builder /workspace/target/release/auth-bridge /bin/auth-bridge

ENTRYPOINT /bin/auth-bridge