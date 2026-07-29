FROM rust:1.97.1 AS chef
RUN cargo install cargo-chef
WORKDIR /app
RUN apt-get update && apt-get install lld clang -y

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin actix_hello

FROM debian:trixie-slim AS runtime
WORKDIR /app

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/actix_hello actix_hello

COPY configuration configuration
ENV APP_ENVIRONMENT production
ENTRYPOINT ["./actix_hello"]

