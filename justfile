# Desktop/native Taidan only; does not require Node or server-oobe.
dev:
  SLINT_LIVE_PREVIEW=1 cargo run --features slint/live-preview

build-desktop:
  cargo build --release -p taidan

test-desktop:
  cargo test --workspace --lib

# Server OOBE is an optional sibling application.
install-server-oobe:
  pnpm --dir server-oobe install --frozen-lockfile

build-server-oobe: install-server-oobe build-server-oobe-backend
  pnpm --dir server-oobe build

# Build the optional libtaidan-backed validation bridge used by the packaged OOBE.
build-server-oobe-backend:
  cargo build --release -p tdnx --no-default-features --features server-oobe

check-server-oobe: install-server-oobe
  pnpm --dir server-oobe check

test-server-oobe: install-server-oobe
  pnpm --dir server-oobe test:accessibility

build-all: build-desktop build-server-oobe
