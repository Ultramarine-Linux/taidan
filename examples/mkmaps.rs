use std::collections::HashSet;

use serde_json::Value;

// src/timezone.rs and ui/pages/0055_timezone.slint assume the same edges
const NORTH: f64 = 84.;
const SOUTH: f64 = -58.;
const MAP_WIDTH_DEGREES: u32 = 360;
const MAP_HEIGHT_DEGREES: u32 = 142;
const SVG_PIXELS_PER_DEGREE: u32 = 4;
const SIMPLIFY_TOLERANCE_DEGREES: f64 = 0.2;

type Point = (f64, f64);

fn lon_lat_rings(geojson: &Value, outer_only: bool) -> Vec<Vec<Point>> {
    let point = |p: &Value| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?));
    let ring = |r: &Value| r.as_array().map(|r| r.iter().filter_map(point).collect::<Vec<_>>());
    let mut out = vec![];
    for feature in geojson["features"].as_array().into_iter().flatten() {
        let geometry = &feature["geometry"];
        let coords = &geometry["coordinates"];
        let polygons = match geometry["type"].as_str() {
            Some("Polygon") => vec![coords],
            Some("MultiPolygon") => coords.as_array().into_iter().flatten().collect(),
            _ => continue,
        };
        for polygon in polygons {
            let polygon = polygon.as_array().into_iter().flatten();
            let take = if outer_only { 1 } else { usize::MAX };
            out.extend(polygon.take(take).filter_map(ring));
        }
    }
    out
}

fn project_clamped((lon, lat): Point) -> String {
    format!("{:.1} {:.1}", lon + 180., NORTH - lat.clamp(SOUTH, NORTH))
}

fn svg_path(points: &[Point], close: bool) -> Option<String> {
    let mut coords = points.iter().copied().map(project_clamped).collect::<Vec<_>>();
    coords.dedup();
    if coords.len() < if close { 3 } else { 2 } {
        return None;
    }
    Some(format!("M{}{}", coords.join(" L"), if close { "Z" } else { "" }))
}

fn svg(comment: &str, path_attrs: &str, d: &str) -> String {
    let (w, h) = (MAP_WIDTH_DEGREES, MAP_HEIGHT_DEGREES);
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{}\" height=\"{}\">\n\
         <!-- {comment}, equirectangular, {NORTH}N to {}S -->\n<path {path_attrs} d=\"{d}\"/>\n</svg>\n",
        w * SVG_PIXELS_PER_DEGREE,
        h * SVG_PIXELS_PER_DEGREE,
        -SOUTH,
    )
}

fn land_svg(geojson: &Value) -> String {
    let is_south_of_map = |r: &&Vec<Point>| r.iter().all(|&(_, lat)| lat < SOUTH);
    let d = lon_lat_rings(geojson, true)
        .iter()
        .filter(|r| !is_south_of_map(r))
        .filter_map(|r| svg_path(r, true))
        .collect::<String>();
    svg("land from Natural Earth 1:110m (public domain)", "fill=\"#000\"", &d)
}

fn douglas_peucker(points: &[Point], tolerance: f64) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let ((x1, y1), (x2, y2)) = (points[0], points[points.len() - 1]);
    let (dx, dy) = (x2 - x1, y2 - y1);
    let norm = dx.hypot(dy);
    #[expect(clippy::suboptimal_flops, reason = "the same arithmetic as the maps were made with")]
    let dist = |&(x, y): &Point| {
        if norm == 0. {
            (x - x1).hypot(y - y1)
        } else {
            (dy * x - dx * y + x2 * y1 - y2 * x1).abs() / norm
        }
    };
    let (index, farthest) = points[1..points.len() - 1]
        .iter()
        .enumerate()
        .map(|(i, p)| (i + 1, dist(p)))
        .fold((0, -1.), |best, cur| if cur.1 > best.1 { cur } else { best });
    if farthest <= tolerance {
        return vec![(x1, y1), (x2, y2)];
    }
    let mut left = douglas_peucker(&points[..=index], tolerance);
    left.pop();
    left.extend(douglas_peucker(&points[index..], tolerance));
    left
}

#[expect(clippy::float_cmp, reason = "edges are at exactly +/-180° or the clamped latitudes")]
fn traces_map_frame(a: Point, b: Point) -> bool {
    (a.0.abs() >= 180. - 1e-6 && a.0 == b.0) || (a.1 == b.1 && (a.1 >= NORTH || a.1 <= SOUTH))
}

fn zone_borders_svg(geojson: &Value) -> String {
    #[expect(clippy::cast_possible_truncation, reason = "a rounded coordinate")]
    let key = |(x, y): Point| ((x * 1e4).round() as i64, (y * 1e4).round() as i64);
    let mut drawn_edges = HashSet::new();
    let mut lines = vec![];
    for ring in lon_lat_rings(geojson, false) {
        let points =
            ring.iter().map(|&(lon, lat)| (lon, lat.clamp(SOUTH, NORTH))).collect::<Vec<_>>();
        let mut line = vec![];
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let edge = if key(a) <= key(b) { (key(a), key(b)) } else { (key(b), key(a)) };
            if a == b || traces_map_frame(a, b) || !drawn_edges.insert(edge) {
                let done = std::mem::take(&mut line);
                if done.len() > 1 {
                    lines.push(done);
                }
                continue;
            }
            if line.is_empty() {
                line.push(a);
            }
            line.push(b);
        }
        if line.len() > 1 {
            lines.push(line);
        }
    }
    let d = lines
        .iter()
        .filter_map(|line| svg_path(&douglas_peucker(line, SIMPLIFY_TOLERANCE_DEGREES), false))
        .collect::<String>();
    svg(
        "time zone boundaries from Natural Earth 1:10m (public domain)",
        "fill=\"none\" stroke=\"#000\" stroke-width=\"0.35\" stroke-linejoin=\"round\" stroke-linecap=\"round\"",
        &d,
    )
}

fn read(path: &str) -> Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} isn't JSON: {e}"))
}

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let [land_json, zones_json, out_dir] = args.as_slice() else {
        eprintln!("usage: mkmaps <ne_110m_land.geojson> <ne_10m_time_zones.geojson> <out dir>");
        std::process::exit(2);
    };
    for (name, svg) in [
        ("world-map.svg", land_svg(&read(land_json))),
        ("world-map-zones.svg", zone_borders_svg(&read(zones_json))),
    ] {
        let out = std::path::Path::new(out_dir).join(name);
        std::fs::write(&out, &svg)
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", out.display()));
        println!("{}: {} bytes", out.display(), svg.len());
    }
}
