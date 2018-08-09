FROM fedora:latest as build

COPY ./ ./
# TODO: openssl-sys is missing
RUN dnf install rust cargo openssl-devel -y

RUN cargo build --release
RUN mkdir -p /build-out
RUN cp target/release/lunch-bot /build-out/

FROM fedora:latest

COPY --from=build /build-out/lunch-bot /
CMD ["/lunch-bot"]
