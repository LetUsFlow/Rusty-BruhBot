# Nightly is required until `-Z sparse-registry` is stabilized in Rust 1.68
# https://github.com/rust-lang/cargo/issues/9069#issuecomment-1408773982
FROM rustlang/rust:nightly-slim as build
# FROM rust:1-slim AS build
WORKDIR /app
COPY . /app

RUN apt-get update && \
    apt-get install -y upx libopus-dev cmake
RUN cargo build --release -Z sparse-registry #&& \
    upx --lzma --best /app/target/release/rusty-bruhbot

FROM debian:stable-slim
WORKDIR /app

RUN apt-get update && \
    apt-get install -y ffmpeg

COPY --from=build /app/target/release/rusty-bruhbot /app/rusty-bruhbot

USER 1000

CMD [ "/app/rusty-bruhbot" ]
