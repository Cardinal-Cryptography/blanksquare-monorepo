#!/bin/bash
set -euo pipefail

# Find repository root and change to it
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR" && git rev-parse --show-toplevel)"
pushd "$REPO_ROOT" >/dev/null

# Configuration
DOCKER_IMAGE="${ECR_IMAGE:-public.ecr.aws/p5g6f1p8/shielder-scheduler:latest}"
EIF_OUTPUT="${COMPONENT_NAME:-shielder-scheduler-tee}.eif"

# Build our custom nitro-cli image if it doesn't exist
NITRO_CLI_IMAGE="shielder-nitro-cli:latest"
DOCKERFILE_PATH=".github/scripts/nitro-cli-docker/Dockerfile.nitro-cli"
if ! docker image inspect "$NITRO_CLI_IMAGE" &>/dev/null; then
    echo "Building custom nitro-cli Docker image..."
    docker build -f "$DOCKERFILE_PATH" -t "$NITRO_CLI_IMAGE" .github/scripts/nitro-cli-docker/
fi

# need to pull docker image inside the container, hence we allow access to the docker socket
docker run --rm \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v "$REPO_ROOT:/workspace" \
  -w /workspace \
  "$NITRO_CLI_IMAGE" \
  "$DOCKER_IMAGE" "$EIF_OUTPUT"

if [ -f "$EIF_OUTPUT" ]; then
    echo "Build completed successfully!"
else
    echo "Build failed - EIF file not found"
    exit 1
fi
