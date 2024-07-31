FROM rust:latest as builder
WORKDIR /usr/src/partialzip
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
LABEL org.opencontainers.image.source=https://github.com/marcograss/partialzip
LABEL org.opencontainers.image.description="Container for rust partialzip"
LABEL org.opencontainers.image.licenses=MIT
RUN apt-get update && apt-get -y upgrade && apt-get -y install curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/partialzip /usr/local/bin/partialzip
ENTRYPOINT [ "partialzip" ]
