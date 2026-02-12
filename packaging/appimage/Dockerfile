FROM ubuntu:20.04

ENV DEBIAN_FRONTEND=noninteractive

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libasound2-dev \
    libdbus-1-dev \
    libxdo-dev \
    libxkbcommon-dev \
    libxcb1-dev \
    libfuse2 \
    file \
    wget \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /build

# Copy source
COPY . .

# Build
ARG BUILD_ARGS=""
RUN ./scripts/build_appimage.sh --no-container $BUILD_ARGS

# The output AppImage will be in the /build directory
