### Builder stage
FROM rust:1.53 AS builder

# Working directory in Docker container
WORKDIR /app

RUN cargo install --locked --branch master \
    --git https://github.com/eeff/cargo-build-deps

# Build the dependencies
COPY Cargo.toml Cargo.lock ./
RUN cargo build-deps --release

# Copy all files from the working environment to the Docker image 
COPY . .

# Set environment variable so that sqlx works in offline mode
ENV SQLX_OFFLINE true

# Build the binary
RUN cargo build --release --bin newsletter

# Strip debug symbols from binary
RUN strip /app/target/release/newsletter

### Runtime stage (not required to have any of the rust toolchain to run the binary)
FROM gcr.io/distroless/cc AS runtime

# Working directory in Docker container
WORKDIR /app

# Copy the compiled binary from the builder environment to the runtime environment
COPY --from=builder /app/target/release/newsletter newsletter

# Copy the configuration files
COPY configuration configuration

# Set production configuration
ENV APP_ENVIRONMENT production

# When `docker run` is executed, launch the binary!
ENTRYPOINT ["./newsletter"]