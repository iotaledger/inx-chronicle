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
RUN apt-get update && apt-get install -y protobuf-compiler
COPY --from=planner /inx-chronicle/recipe.json recipe.json

RUN cargo chef cook --profile production --recipe-path recipe.json
COPY . .
RUN cargo build --profile production --bin inx-chronicle

############################
# Image
############################
# https://console.cloud.google.com/gcr/images/distroless/global/cc-debian11
# using distroless cc "nonroot" image, which includes everything in the base image (glibc, libssl and openssl)
FROM gcr.io/distroless/cc-debian11:nonroot AS runtime

# REST API
EXPOSE 8042/tcp
# Metrics
EXPOSE 9100/tcp

COPY --chown=nonroot:nonroot --from=builder /inx-chronicle/target/production/inx-chronicle /app/inx-chronicle

WORKDIR /app
USER nonroot

ENTRYPOINT ["/app/inx-chronicle"]
