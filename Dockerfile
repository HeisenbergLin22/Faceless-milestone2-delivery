FROM rust:1.65.0

# RUN apt-get update && apt-get -y upgrade \
#    && apt-get install -y cmake libclang-dev libssl-dev protobuf-compiler
RUN apt-get update \
   && apt-get install -y cmake libclang-dev libssl-dev protobuf-compiler

RUN rustup update \
    && rustup update nightly \
    && rustup target add wasm32-unknown-unknown --toolchain nightly \
    && rustup target add wasm32-unknown-unknown

WORKDIR /home/workspace/faceless
COPY . .

RUN cd ./faceless-substrate-node \
    && cargo build --release

EXPOSE 9944

