FROM rust:latest

RUN sed -i 's@deb.debian.org@mirrors.cloud.tencent.com@g' /etc/apt/sources.list.d/debian.sources

# Install additional development tools and dependencies
RUN apt-get update && \
    apt-get install -y \
            cmake clang llvm-dev libclang-dev \
            protobuf-compiler \
            wget unzip lsof nload htop net-tools dnsutils openssh-server vim && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Install Rust development VS Code extensions for code-server
RUN curl -fsSL https://code-server.dev/install.sh | sh \
    && code-server --install-extension rust-lang.rust-analyzer \
    && code-server --install-extension vadimcn.vscode-lldb \
    && code-server --install-extension serayuzgur.crates \
    && code-server --install-extension formulahendry.code-runner \
    && code-server --install-extension tamasfe.even-better-toml \
    && code-server --install-extension Tencent-Cloud.coding-copilot

ENV RUST_BACKTRACE=1
ENV CARGO_INCREMENTAL=1

# 指定字符集支持命令行输入中文（根据需要选择字符集）
ENV LANG C.UTF-8
ENV LANGUAGE C.UTF-8