FROM rust:1-alpine AS build
WORKDIR /app
COPY . /app

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN rustup toolchain add nightly && \
    rustup default nightly
RUN apk add upx musl-dev pkgconf opus-dev
RUN cargo build --release #&& \
    upx --lzma --best /app/target/release/rusty-bruhbot

FROM alpine:latest
WORKDIR /app

RUN apk add ffmpeg && \
    rm -fr /var/cache/apk/*

COPY --from=build /app/target/release/rusty-bruhbot /app/rusty-bruhbot

#USER 1000

CMD [ "/app/rusty-bruhbot" ]
