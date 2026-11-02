dev:
  SLINT_LIVE_PREVIEW=1 cargo run --features slint/live-preview

build-desktop:
  cargo build --release -p taidan

test-desktop:
  cargo test --workspace --lib --bins

# libtaidan validation bridge used by ultramarine-server-oobe
# (https://github.com/Ultramarine-Linux/server-oobe)
build-tdnx:
  cargo build --release -p tdnx --no-default-features --features server-oobe

# regenerates the time zone page's maps from Natural Earth (public domain)
maps:
  #!/usr/bin/env bash
  set -euo pipefail
  dir=$(mktemp -d)
  trap 'rm -rf "$dir"' EXIT
  ne=https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson
  curl -sSfL -o "$dir/land.geojson" "$ne/ne_110m_land.geojson"
  curl -sSfL -o "$dir/zones.geojson" "$ne/ne_10m_time_zones.geojson"
  cargo run --release --example mkmaps -- "$dir/land.geojson" "$dir/zones.geojson" data/icons

# bumps taidan and taidan_ui to a version, or by `major`, `minor`, or `patch`. Doesn't commit or tag.
# needs cargo-edit (`cargo install cargo-edit`)
bump to="patch":
  #!/usr/bin/env bash
  set -euo pipefail
  case "{{to}}" in
    major|minor|patch) cargo set-version -p taidan --bump "{{to}}" ;;
    *) cargo set-version -p taidan "{{to}}" ;;
  esac
  cargo update --workspace --offline
