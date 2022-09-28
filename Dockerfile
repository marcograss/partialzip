FROM rust:latest as builder
WORKDIR /usr/src/partialzip
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt-get update && apt-get -y upgrade && apt-get -y install curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/partialzip /usr/local/bin/partialzip
ENTRYPOINT [ "partialzip" ]
