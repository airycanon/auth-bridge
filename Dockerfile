FROM rust:bookworm AS builder

WORKDIR /workspace

COPY ./ /workspace

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      git \
      build-essential \
      cmake \
      libclang-dev && \
    rm -rf /var/lib/apt/lists/* && \
    cargo build --release


FROM debian:bookworm-slim

COPY --from=builder /workspace/target/release/auth-bridge /bin/auth-bridge

ENTRYPOINT ["/bin/auth-bridge"]
