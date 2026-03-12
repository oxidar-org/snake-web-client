# Stage 1: Build Rust → WASM
FROM rust:1.85-slim AS rust-builder

RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

WORKDIR /app
COPY rust/ rust/

RUN wasm-pack build rust --target web --out-dir /app/js/src/wasm

# Stage 2: Build JS → static files
FROM node:22-slim AS js-builder

WORKDIR /app
COPY js/ js/

# Copy WASM output from previous stage
COPY --from=rust-builder /app/js/src/wasm js/src/wasm

WORKDIR /app/js
RUN yarn install --frozen-lockfile
RUN yarn build

# Stage 3: Export stage — static files only
FROM scratch AS final

COPY --from=js-builder /app/js/dist /dist/
