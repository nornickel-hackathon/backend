FROM rust:1-bookworm AS build

WORKDIR /app/backend
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release -p platform

FROM debian:bookworm-slim

ENV NORNIKEL_ROOT=/data/docs \
    BIND_ADDR=0.0.0.0:8080

WORKDIR /app
COPY --from=build /app/backend/target/release/platform /usr/local/bin/platform

EXPOSE 8080
CMD ["platform"]
