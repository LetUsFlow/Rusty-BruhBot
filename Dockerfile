# Nightly is required until `-Z sparse-registry` is stabilized in Rust 1.68
# https://github.com/rust-lang/cargo/issues/9069#issuecomment-1408773982
FROM rustlang/rust:nightly-slim as build
# FROM rust:1-slim AS build
WORKDIR /app
COPY . /app
COPY --from=mwader/static-ffmpeg:5.1.2 /ffmpeg /ffmpeg

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN apt-get update && \
    apt-get install -y upx libopus-dev cmake
RUN cargo build --release && \
    upx --lzma --best /app/target/release/rusty-bruhbot && \
    upx -1 /ffmpeg

FROM gcr.io/distroless/cc:nonroot
WORKDIR /app

COPY --from=build /app/target/release/rusty-bruhbot /app/rusty-bruhbot
COPY --from=build /ffmpeg /bin/

USER nonroot

CMD [ "/app/rusty-bruhbot" ]
