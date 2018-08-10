FROM fedora:latest

COPY ./ ./
# TODO: openssl-sys is missing
RUN dnf install rust cargo openssl-devel -y && \
    cargo build --release                   && \
    mkdir -p /build-out                     && \
    cp target/release/lunch-bot /           && \
    rm -rf target/

CMD ["/lunch-bot"]
