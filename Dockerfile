### Planner stage
FROM lukemathwalker/cargo-chef as planner

WORKDIR /app
COPY . .

# Compute a lock-like file for the project
RUN cargo chef prepare  --recipe-path recipe.json

### Cacher stage
FROM lukemathwalker/cargo-chef as cacher

WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json

# Build project dependencies, not the application
RUN cargo chef cook --release --recipe-path recipe.json

### Builder stage
FROM rust AS builder

# Working directory in Docker container
WORKDIR /app

# Copy over the cached dependencies
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo

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