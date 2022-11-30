FROM rust:latest as build

RUN apt-get update && apt-get install -y clang

# Cache dependencies
RUN USER=root cargo new --bin zeropool-faucet
WORKDIR /zeropool-faucet
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

COPY --from=build /zeropool-faucet/target/release/zeropool-faucet .
CMD ["./zeropool-faucet"]
