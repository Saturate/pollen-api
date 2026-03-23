# Build stage
FROM rust:1.85-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev cmake make perl gcc g++

# Create a new empty project
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Copy source code
COPY src ./src

# Build for release
RUN cargo build --release

# Runtime stage
FROM gcr.io/distroless/static:nonroot

# Copy the binary from builder
COPY --from=builder /app/target/release/pollen-api /pollen-api

# Expose port
EXPOSE 3060

# Run the binary
ENTRYPOINT ["/pollen-api"]
