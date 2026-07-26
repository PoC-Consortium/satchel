# Build from the REPO ROOT (crier path-depends on ../pact-nostr, ../pact-proto):
#   docker build -t crier -f crier/Dockerfile .
FROM rust:1-bookworm AS build
WORKDIR /src
COPY pact-proto ./pact-proto
COPY pact-nostr ./pact-nostr
COPY crier ./crier
RUN cargo build --release --manifest-path crier/Cargo.toml

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --home /data crier
COPY --from=build /src/crier/target/release/crier /usr/local/bin/crier
USER crier
WORKDIR /data
VOLUME /data
# /data holds crier.toml (optional) and crier-state.json.
ENTRYPOINT ["/usr/local/bin/crier"]
