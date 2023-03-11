FROM rust:1-slim-bullseye as builder
WORKDIR /app
COPY . /app
COPY --from=mwader/static-ffmpeg:6.0 /ffmpeg /ffmpeg

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN apt-get update && \
    apt-get install -y upx libopus-dev cmake
RUN cargo build --release && \
    upx --lzma --best /app/target/release/rusty-bruhbot && \
    upx -1 /ffmpeg

FROM gcr.io/distroless/cc:nonroot

COPY --from=builder /app/target/release/rusty-bruhbot /bin/
COPY --from=builder /ffmpeg /bin/

USER nonroot

CMD [ "rusty-bruhbot" ]
