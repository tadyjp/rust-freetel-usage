FROM rust:1.19.0
RUN apt-get update && apt-get install -y pkg-config libssl-dev
RUN mkdir -p /opt/rust
WORKDIR /opt/rust
CMD ["bash"]
