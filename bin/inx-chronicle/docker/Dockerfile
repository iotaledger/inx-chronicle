############################
# Build
############################
FROM rust:1-buster as build

LABEL org.label-schema.description="IOTA permanode implemented as an IOTA Node Extension (INX)."
LABEL org.label-schema.name="iotaledger/inx-chronicle"
LABEL org.label-schema.schema-version="1.0"
LABEL org.label-schema.vcs-url="https://github.com/iotaledger/inx-chronicle"
LABEL org.label-schema.usage="https://github.com/iotaledger/inx-chronicle/blob/main/README.md"

WORKDIR /inx-chronicle
COPY . .

RUN cargo build --release

############################
# Image
############################
# https://console.cloud.google.com/gcr/images/distroless/global/cc-debian11
# using distroless cc "nonroot" image, which includes everything in the base image (glibc, libssl and openssl)
FROM gcr.io/distroless/cc-debian11:nonroot

# EXPOSE 9091/tcp
# EXPOSE 9312/tcp

COPY --chown=nonroot:nonroot --from=build /inx-chronicle/target/release/inx-chronicle /app/inx-chronicle

WORKDIR /app
USER nonroot

ENTRYPOINT ["/app/inx-chronicle"]
