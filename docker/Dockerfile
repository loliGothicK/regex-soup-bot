FROM rust:1.55.0 as builder

WORKDIR /work

# we need to copy rust-toolchain.toml because we are using a nightly build
COPY /src /work/src
COPY /Cargo.lock /Cargo.toml /rust-toolchain.toml /work/

RUN cargo build --release

FROM phusion/baseimage:focal-1.1.0
COPY --from=builder /work/target/release/regexsoup /usr/local/bin/regexsoup

RUN mkdir /etc/service/regexsoup
COPY docker/regexsoup.sh /etc/service/regexsoup/run
RUN chmod +x /etc/service/regexsoup/run
