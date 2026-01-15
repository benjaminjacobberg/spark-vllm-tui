# Build Stage
FROM rust:1.84-bookworm as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev cmake && rm -rf /var/lib/apt/lists/*

# Copy source code
COPY . .

# Build release binary
RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/vllm_tui /app/vllm_tui

# Copy configuration templates
COPY models.json /app/
COPY server.template.json /app/

# Create a place for the actual config to live
# User will need to mount server.json here
# We can't really "template" it at runtime easily without valid values, 
# so we rely on the user providing it or copying the template.

CMD ["./vllm_tui"]
