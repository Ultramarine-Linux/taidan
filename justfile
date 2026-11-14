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

# regenerate the time zone maps from Natural Earth
maps:
  #!/usr/bin/env bash
  set -euo pipefail
  dir=$(mktemp -d)
  trap 'rm -rf "$dir"' EXIT
  natural_earth=https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson
  curl -sSfL -o "$dir/land.geojson" "$natural_earth/ne_110m_land.geojson"
  curl -sSfL -o "$dir/zones.geojson" "$natural_earth/ne_10m_time_zones.geojson"
  cargo run --release --example mkmaps -- "$dir/land.geojson" "$dir/zones.geojson" data/icons

# set the version, or bump by major, minor or patch (needs cargo-edit)
bump to="patch":
  #!/usr/bin/env bash
  set -euo pipefail
  case "{{to}}" in
    major|minor|patch) cargo set-version -p taidan --bump "{{to}}" ;;
    *) cargo set-version -p taidan "{{to}}" ;;
  esac
  cargo update --workspace --offline
