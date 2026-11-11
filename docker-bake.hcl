# Multi-architecture server image.
#
#   docker buildx bake --print server
#   docker buildx build --platform linux/amd64,linux/arm64 \
#     -t registry.example/ion-of-notes-server:1.0.0 --push .
#
# `bake` without `--push` cannot load two platforms into the local Docker
# image store; use `--print` to validate or push to a registry.

target "server" {
  context    = "."
  dockerfile = "Dockerfile"
  tags       = ["ion-of-notes-server:local"]
  platforms  = ["linux/amd64", "linux/arm64"]
}
