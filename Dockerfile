### BUILD STAGE ###
FROM rust:1.93-bookworm as builder

WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev cmake && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release

### RUNTIME STAGE ###
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/vllm_tui /app/vllm_tui
COPY models.json /app/
COPY server.template.json /app/

# Mount server.json at runtime or copy server.template.json to server.json and fill in values

CMD ["./vllm_tui"]