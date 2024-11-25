FROM rust:1.82.0 as build
WORKDIR /usr/src/myapp
COPY . .
WORKDIR /usr/src/myapp/implementation
RUN cargo build --release

FROM rust:1.82.0
EXPOSE 8080
WORKDIR /usr/src/implementation
COPY --from=build /usr/src/myapp/implementation/target/release/implementation .
CMD ["./implementation"]
