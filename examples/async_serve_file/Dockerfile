FROM ubuntu:mantic

RUN apt-get update
RUN DEBIAN_FRONTEND=noninteractive \
    apt-get install -y --no-install-recommends rust-all git haproxy ca-certificates

COPY . /root/haproxy
WORKDIR /root/haproxy

# Build plugin
RUN sed -rie 's/^(haproxy-api)\s=\s.+$/\1 = "*"/g' Cargo.toml \
    && cargo build --release

CMD ["haproxy", "-d", "-f", "haproxy.cfg"]
