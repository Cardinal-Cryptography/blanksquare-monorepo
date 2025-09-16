#!/bin/bash
set -e

# Entry point script for the nitro-cli Docker container

# Handle direct nitro-cli commands (like --version)
if [ "$1" = "--version" ] || [ "$1" = "--help" ]; then
    exec nitro-cli "$@"
fi

DOCKER_IMAGE="${1:-public.ecr.aws/p5g6f1p8/shielder-scheduler:latest}"
EIF_OUTPUT="${2:-shielder-scheduler-tee.eif}"

echo "Building EIF from Docker image: $DOCKER_IMAGE"
echo "Output file: $EIF_OUTPUT"

# Build the EIF
nitro-cli build-enclave \
    --docker-uri "$DOCKER_IMAGE" \
    --output-file "$EIF_OUTPUT"

echo "EIF built successfully: $EIF_OUTPUT"

# Show the build info
if [ -f "$EIF_OUTPUT" ]; then
    echo "EIF file size: $(du -h $EIF_OUTPUT | cut -f1)"
    nitro-cli describe-eif --eif-path "$EIF_OUTPUT" > eif-info.json
    echo "PCR values saved to: eif-info.json"
    cat eif-info.json
fi
