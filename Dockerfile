FROM rust:1.82.0 AS build
RUN rustup target add x86_64-unknown-linux-musl
RUN apt-get update && apt-get install -y musl-tools
WORKDIR /usr/src/myapp
COPY . .
WORKDIR /usr/src/myapp/implementation
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM alpine:latest
WORKDIR /usr/src/implementation
COPY --from=build /usr/src/myapp/implementation/target/x86_64-unknown-linux-musl/release/implementation .
EXPOSE 8080
ENTRYPOINT ["./implementation"]
