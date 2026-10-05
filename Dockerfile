FROM rust:1-alpine AS builder
WORKDIR /app
COPY . /app

RUN apk add upx cmake make
RUN cargo build --release && \
    upx --lzma --best /app/target/release/rusty-bruhbot

FROM gcr.io/distroless/static:nonroot
COPY --from=builder /app/target/release/rusty-bruhbot /bin/
USER nonroot

CMD [ "rusty-bruhbot" ]
