############################
# Build
############################
FROM lukemathwalker/cargo-chef:latest-rust-buster AS chef

LABEL org.label-schema.description="IOTA permanode implemented as an IOTA Node Extension (INX)."
LABEL org.label-schema.name="iotaledger/inx-chronicle"
LABEL org.label-schema.schema-version="1.0"
LABEL org.label-schema.vcs-url="https://github.com/iotaledger/inx-chronicle"
LABEL org.label-schema.usage="https://github.com/iotaledger/inx-chronicle/blob/main/README.md"

WORKDIR /inx-chronicle

FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /inx-chronicle/recipe.json recipe.json

RUN apt-get update && apt-get install -y protobuf-compiler

RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin inx-chronicle

############################
# Image
############################
# https://console.cloud.google.com/gcr/images/distroless/global/cc-debian11
# using distroless cc "nonroot" image, which includes everything in the base image (glibc, libssl and openssl)
FROM gcr.io/distroless/cc-debian11:nonroot AS runtime

COPY --chown=nonroot:nonroot --from=builder /inx-chronicle/target/release/inx-chronicle /app/inx-chronicle

WORKDIR /app
USER nonroot

ENTRYPOINT ["/app/inx-chronicle"]
