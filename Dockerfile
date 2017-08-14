FROM fnichol/rust:1.15.1
RUN mkdir -p /opt/rust
WORKDIR /opt/rust
CMD ["bash"]
