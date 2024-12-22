FROM lukemathwalker/cargo-chef:0.1.68-rust-bookworm as chef
WORKDIR /app
# Change mirror to KKU to speed up resolving
RUN sed -i 's/deb\.debian\.org/mirror.kku.ac.th/g' /etc/apt/sources.list.d/debian.sources
# KKU don't have debian-security
RUN sed -i 's/mirror\.kku\.ac\.th\/debian-security/mirror.sg.gs\/debian-security/g' /etc/apt/sources.list.d/debian.sources
# RUN cat /etc/apt/sources.list.d/debian.sources
RUN apt-get update && apt-get install lld clang -y

FROM chef as planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef as builder
ENV SQLX_OFFLINE true
COPY --from=planner /app/recipe.json recipe.json
# Build project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
# Build project
RUN cargo build --release --bin server

FROM debian:bookworm-slim AS runtime

# Set the working directory
WORKDIR /app

# Change mirror to KKU to speed up resolving
RUN sed -i 's/deb\.debian\.org/mirror.kku.ac.th/g' /etc/apt/sources.list.d/debian.sources
# KKU don't have debian-security
RUN sed -i 's/mirror\.kku\.ac\.th\/debian-security/mirror.sg.gs\/debian-security/g' /etc/apt/sources.list.d/debian.sources
# Install runtime dependencies
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends ca-certificates \
    # Clean up to keep the image size small
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/server server

# Set environment variables
ENV PORT 8080
ENV APP_ENVIRONMENT production
ENV RUST_LOG server=info,tower_http=info,sqlx=info

# Expose the port your app runs on
EXPOSE 8080

# Run the binary
ENTRYPOINT ["./server"]
