#!/bin/bash
set -e

# Configuration
DOCKER_IMAGE="${ECR_IMAGE:-public.ecr.aws/p5g6f1p8/shielder-scheduler:latest}"
EIF_OUTPUT="shielder-scheduler-tee.eif"

# Build our custom nitro-cli image if it doesn't exist
NITRO_CLI_IMAGE="shielder-nitro-cli:latest"
if ! docker image inspect "$NITRO_CLI_IMAGE" &>/dev/null; then
    echo "Building custom nitro-cli Docker image..."
    docker build -f Dockerfile.nitro-cli -t "$NITRO_CLI_IMAGE" .
fi

# Use our custom image to build the EIF
echo "Using custom nitro-cli Docker image..."
# need to pull docker image inside the container, hence we allow access to the docker socket
docker run --rm \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v "$(pwd):/workspace" \
  -w /workspace \
  "$NITRO_CLI_IMAGE" \
  "$DOCKER_IMAGE" "$EIF_OUTPUT"

if [ -f "$EIF_OUTPUT" ]; then
    echo "Build completed successfully!"
else
    echo "Build failed - EIF file not found"
    exit 1
fi
