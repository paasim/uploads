FROM rust:1.85-bookworm

ARG GROUP_ID
ARG USER_ID

RUN dpkg --add-architecture arm64 \
  && apt update \
  && apt install -y pkg-config crossbuild-essential-arm64 libssl-dev libssl-dev:arm64 \
  && groupadd -g ${GROUP_ID} builder \
  && useradd -rm -d /home/builder -s /bin/bash -u ${USER_ID} -g ${GROUP_ID} builder

USER builder
WORKDIR /home/builder/rust-pkg

RUN rustup target add aarch64-unknown-linux-gnu \
  && echo '[target.aarch64-unknown-linux-gnu]' > /usr/local/cargo/config.toml \
  && echo 'linker = "/usr/bin/aarch64-linux-gnu-gcc"' >> /usr/local/cargo/config.toml
ENV PKG_CONFIG_SYSROOT_DIR="/usr/lib/aarch64-linux-gnu/pkgconfig"

CMD ["cargo", "build", "-r", "--target", "aarch64-unknown-linux-gnu"]
