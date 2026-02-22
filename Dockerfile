FROM rust:latest

WORKDIR /build
COPY . .
RUN cargo build --release --features nats 2>&1

RUN cp target/release/sysops-agent /usr/local/bin/sysops-agent
COPY config-docker.toml /etc/sysops-agent/config.toml

CMD ["/usr/local/bin/sysops-agent", "--config", "/etc/sysops-agent/config.toml"]
