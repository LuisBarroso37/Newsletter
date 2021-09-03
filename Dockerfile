### Planner stage
FROM rust:1.54.0 as planner

WORKDIR /app
RUN cargo install cargo-chef 
COPY . .

# Compute a lock-like file for the project
RUN cargo chef prepare  --recipe-path recipe.json

### Cacher stage
FROM rust:1.54.0 as builder

WORKDIR /app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json

# Build project dependencies, not the application
RUN cargo chef cook --release --recipe-path recipe.json

# Copy all files from the working environment to the Docker image 
COPY . .

# Set environment variable so that sqlx works in offline mode
# Used for compile time checks
ENV SQLX_OFFLINE true

# Build the binary
RUN cargo build --release --bin newsletter

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