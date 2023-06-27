FROM rust:1.70.0 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

# Copy the project to a target independent location. This will be used by the Runtime image
RUN cp target/release/watch-release .

# Runtime Image
FROM debian:bullseye-slim
WORKDIR /app

RUN set -ex; \ 
    apt-get update; \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/watch-release .
ENTRYPOINT []