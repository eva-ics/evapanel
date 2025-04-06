all:
  @echo "Select target"

docker-image-x86_64:
  docker build -t evapanel-build-env-x86_64:latest -f docker.cross/Dockerfile.cross.x86_64 .

linux-x86_64:
  cross build --target x86_64-unknown-linux-gnu --release

docker-image-aarch64:
  docker build -t evapanel-build-env-aarch64:latest -f docker.cross/Dockerfile.cross.aarch64 .

linux-aarch64:
  cross build --target aarch64-unknown-linux-gnu --release
