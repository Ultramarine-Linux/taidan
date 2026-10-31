dev:
  SLINT_LIVE_PREVIEW=1 cargo run --features slint/live-preview

build-desktop:
  cargo build --release -p taidan

test-desktop:
  cargo test --workspace --lib

# libtaidan validation bridge used by ultramarine-server-oobe
# (https://github.com/Ultramarine-Linux/server-oobe)
build-tdnx:
  cargo build --release -p tdnx --no-default-features --features server-oobe
