FROM rust:1-slim-bullseye as builder
WORKDIR /app
COPY . /app
ENV LIBOPUS_STATIC=true
RUN apt-get update && \
    apt-get install -y upx libopus-dev cmake
RUN cargo build --release && \
    upx --lzma --best /app/target/release/rusty-bruhbot

FROM gcr.io/distroless/cc:nonroot
COPY --from=builder /app/target/release/rusty-bruhbot /bin/
USER nonroot
CMD [ "rusty-bruhbot" ]
