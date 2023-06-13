FROM rust:latest as build

RUN apt-get update && apt-get install -y clang

# Cache dependencies
WORKDIR /app
RUN USER=root cargo init --bin --name zeropool-faucet
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release

# Build
RUN rm src/*.rs
RUN rm ./target/release/deps/zeropool_faucet*
COPY ./src ./src
RUN cargo build --release

# Final image
FROM rust:latest
WORKDIR /app
COPY --from=build /app/target/release/zeropool-faucet .
CMD ["./zeropool-faucet"]
