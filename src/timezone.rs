//! the time zone page: zones from the system tzdata, shown on a map and searchable
use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, Model, ToSharedString};

use crate::ui::{AppWindow, Lang, TimeZone, Tz};

/// zones with their main cities and coordinates, one per region that's had the same clocks since 1970
const ZONE_TAB: &str = "/usr/share/zoneinfo/zone1970.tab";
/// the latitudes the map covers, see `data/icons/world-map.svg`
const MAP_NORTH: f64 = 84.;
const MAP_SOUTH: f64 = -58.;
/// shown when nothing better is known, and not on the map
const UTC: &str = "UTC";

#[derive(Clone, Debug, PartialEq)]
struct Zone {
    id: String,
    /// ISO 3166 codes, the first is the one the zone is mainly for
    countries: Vec<String>,
    lat: f64,
    lon: f64,
}

/// `±DDMM` or `±DDMMSS` (`±DDDMM…` for longitude) as degrees
fn parse_angle(s: &str, deg_digits: usize) -> Option<f64> {
    let (sign, digits) = s.split_at_checked(1)?;
    let sign = match sign {
        "+" => 1.,
        "-" => -1.,
        _ => return None,
    };
    let part = |range: std::ops::Range<usize>| digits.get(range)?.parse::<f64>().ok();
    let deg = part(0..deg_digits)?;
    let min = part(deg_digits..deg_digits + 2)?;
    let sec =
        if digits.len() > deg_digits + 2 { part(deg_digits + 2..deg_digits + 4)? } else { 0. };
    Some(sign * (deg + min / 60. + sec / 3600.))
}

/// ISO 6709 coordinates as in `zone1970.tab`, e.g. `+4230+00131`
fn parse_coords(s: &str) -> Option<(f64, f64)> {
    let split = s[1..].find(['+', '-'])? + 1;
    let (lat, lon) = s.split_at(split);
    Some((parse_angle(lat, 2)?, parse_angle(lon, 3)?))
}

fn parse_tab(tab: &str) -> Vec<Zone> {
    tab.lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|line| {
            let mut cols = line.split('\t');
            let countries = cols.next()?.split(',').map(str::to_owned).collect();
            let (lat, lon) = parse_coords(cols.next()?)?;
            let id = cols.next()?.to_owned();
            Some(Zone { id, countries, lat, lon })
        })
        .collect()
}

/// `America/Argentina/Buenos_Aires` → `Buenos Aires`
fn city(id: &str) -> String {
    id.rsplit('/').next().unwrap_or(id).replace('_', " ")
}

/// the offset from UTC right now, like `UTC−7` or `UTC+5:30`
fn offset_text(id: &str) -> String {
    let Ok(tz) = jiff::tz::TimeZone::get(id) else { return UTC.to_owned() };
    let secs = tz.to_offset(jiff::Timestamp::now()).seconds();
    if secs == 0 {
        return UTC.to_owned();
    }
    let sign = if secs < 0 { '−' } else { '+' };
    let (h, m) = (secs.abs() / 3600, secs.abs() % 3600 / 60);
    if m == 0 { format!("UTC{sign}{h}") } else { format!("UTC{sign}{h}:{m:02}") }
}

/// the time now in `id`, formatted for `locale`
fn local_time(id: &str, locale: &str) -> String {
    use icu::datetime::{NoCalendarFormatter, fieldsets::T};
    let Ok(tz) = jiff::tz::TimeZone::get(id) else { return String::new() };
    let now = jiff::Timestamp::now().to_zoned(tz).time();
    let locale = icu::locale::Locale::try_from_str(locale).unwrap_or(icu::locale::locale!("en-US"));
    let Ok(fmt) = NoCalendarFormatter::try_new((&locale).into(), T::hm()) else {
        return now.strftime("%H:%M").to_string();
    };
    let Ok(time) = icu::time::Time::try_new(
        now.hour().try_into().unwrap_or_default(),
        now.minute().try_into().unwrap_or_default(),
        0,
        0,
    ) else {
        return now.strftime("%H:%M").to_string();
    };
    fmt.format(&time).to_string()
}

/// country names in the language of `locale`
struct Regions(Option<icu::experimental::displaynames::multi::RegionDisplayNames>);

impl Regions {
    fn new(locale: &str) -> Self {
        use icu::experimental::displaynames::{DisplayNamesOptions, multi::RegionDisplayNames};
        let locale =
            icu::locale::Locale::try_from_str(locale).unwrap_or(icu::locale::locale!("en-US"));
        Self(RegionDisplayNames::try_new((&locale).into(), DisplayNamesOptions::default()).ok())
    }

    fn name(&self, code: &str) -> String {
        let region = icu::locale::subtags::Region::try_from_str(code).ok();
        self.0.as_ref().zip(region).and_then(|(n, r)| n.of(r)).unwrap_or(code).to_owned()
    }
}

fn entry(zone: &Zone, regions: &Regions) -> TimeZone {
    // Antarctic stations are south of the map, they can still be searched for
    let on_map = (MAP_SOUTH..=MAP_NORTH).contains(&zone.lat);
    TimeZone {
        id: zone.id.as_str().into(),
        city: city(&zone.id).into(),
        region: zone.countries.first().map(|c| regions.name(c)).unwrap_or_default().into(),
        offset: offset_text(&zone.id).into(),
        #[expect(clippy::cast_possible_truncation, reason = "a position on screen")]
        x: if on_map { ((zone.lon + 180.) / 360.) as f32 } else { -1. },
        #[expect(clippy::cast_possible_truncation, reason = "a position on screen")]
        y: if on_map { ((MAP_NORTH - zone.lat) / (MAP_NORTH - MAP_SOUTH)) as f32 } else { -1. },
    }
}

fn utc_entry() -> TimeZone {
    TimeZone {
        id: UTC.into(),
        city: UTC.into(),
        region: "".into(),
        offset: UTC.into(),
        x: -1.,
        y: -1.,
    }
}

/// a zone's offsets in January and July this year, which differ where there's daylight saving
fn rules(id: &str) -> Option<(i32, i32)> {
    let tz = jiff::tz::TimeZone::get(id).ok()?;
    let year = jiff::Zoned::now().year();
    let at = |month| {
        let date = jiff::civil::date(year, month, 15).to_zoned(tz.clone()).ok()?;
        Some(date.offset().seconds())
    };
    Some((at(1)?, at(7)?))
}

/// the zones shown on the map: the first, and so main, one of each country that keeps its clocks
/// the same way, the rest differ only in history (e.g. the counties of Indiana) and are still
/// found by searching
fn map_zones(zones: &[Zone]) -> Vec<Zone> {
    let mut seen = std::collections::HashSet::new();
    zones
        .iter()
        .filter(|z| (MAP_SOUTH..=MAP_NORTH).contains(&z.lat))
        .filter(|z| seen.insert((z.countries.first().cloned(), rules(&z.id))))
        .cloned()
        .collect()
}

/// the zone nearest a spot on the map, given as fractions of its width and height
fn nearest(zones: &[Zone], x: f64, y: f64) -> Option<&Zone> {
    let lon = x.mul_add(360., -180.);
    let lat = y.mul_add(-(MAP_NORTH - MAP_SOUTH), MAP_NORTH);
    let dist = |z: &Zone| {
        // the map wraps around at the date line, and longitude shrinks toward the poles
        let dlon = (z.lon - lon).abs();
        let dlon = dlon.min(360. - dlon) * lat.to_radians().cos();
        dlon.mul_add(dlon, (z.lat - lat).powi(2))
    };
    zones
        .iter()
        .filter(|z| (MAP_SOUTH..=MAP_NORTH).contains(&z.lat))
        .min_by(|a, b| dist(a).total_cmp(&dist(b)))
}

/// the zone to start with: the system's, else the main one for the language's country
fn suggest(zones: &[Zone], locale: &str) -> Option<String> {
    let system = jiff::tz::TimeZone::system();
    if let Some(name) = system.iana_name()
        && zones.iter().any(|z| z.id == name)
    {
        return Some(name.to_owned());
    }
    let locale = icu::locale::Locale::try_from_str(locale).ok()?;
    let region = locale.id.region?;
    zones
        .iter()
        .find(|z| z.countries.first().is_some_and(|c| c == region.as_str()))
        .map(|z| z.id.clone())
}

fn matches(tz: &TimeZone, query: &str) -> bool {
    [&tz.city, &tz.region, &tz.id, &tz.offset]
        .iter()
        .any(|s| s.to_lowercase().replace('−', "-").contains(query))
}

struct State {
    zones: Vec<Zone>,
    /// the ones with a dot on the map, see [`map_zones`]
    on_map: Vec<Zone>,
    /// set once the user picks one, so changing the language no longer changes it
    touched: bool,
    query: String,
}

fn locale(ui: &AppWindow) -> String {
    let locale = ui.global::<Lang<'_>>().get_current_language().locale;
    if locale == "en-owo" { "en-US".to_owned() } else { locale.into() }
}

fn select(ui: &AppWindow, id: &str) {
    let tz = ui.global::<Tz<'_>>();
    let current = tz.get_all_zones().iter().find(|z| z.id == id).unwrap_or_else(utc_entry);
    tz.set_current_time(local_time(&current.id, &locale(ui)).into());
    tz.set_current(current);
}

fn refresh(ui: &AppWindow, state: &State) {
    let regions = Regions::new(&locale(ui));
    let mut entries = state.zones.iter().map(|z| entry(z, &regions)).collect::<Vec<_>>();
    let tz = ui.global::<Tz<'_>>();
    let dots = state.on_map.iter().map(|z| entry(z, &regions)).collect::<Vec<_>>();
    tz.set_zones(slint::ModelRc::new(slint::VecModel::from(dots)));
    tz.set_all_zones(slint::ModelRc::new(slint::VecModel::from(entries.clone())));
    entries.push(utc_entry());
    entries.sort_by(|a, b| a.city.cmp(&b.city));
    let query = state.query.clone();
    let results = entries.into_iter().filter(|e| matches(e, &query)).collect::<Vec<_>>();
    tz.set_results(slint::ModelRc::new(slint::VecModel::from(results)));
    let id = if state.touched {
        tz.get_current().id.to_string()
    } else {
        suggest(&state.zones, &locale(ui)).unwrap_or_else(|| UTC.to_owned())
    };
    select(ui, &id);
}

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, tz: &Tz<'_>) {
    let zones = std::fs::read_to_string(ZONE_TAB).map_or_else(
        |e| {
            tracing::error!(?e, "cannot read {ZONE_TAB}");
            vec![]
        },
        |tab| parse_tab(&tab),
    );
    let on_map = map_zones(&zones);
    tracing::debug!(all = zones.len(), on_map = on_map.len(), "Loaded time zones");
    let state =
        Rc::new(RefCell::new(State { zones, on_map, touched: false, query: String::new() }));
    tz.on_refresh({
        let (ui, state) = (ui.clone(), Rc::clone(&state));
        move || {
            if let Some(ui) = ui.upgrade() {
                refresh(&ui, &state.borrow());
            }
        }
    });
    tz.on_search({
        let (ui, state) = (ui.clone(), Rc::clone(&state));
        move |query| {
            let Some(ui) = ui.upgrade() else { return };
            let mut state = state.borrow_mut();
            state.query = query.trim().to_lowercase();
            refresh(&ui, &state);
        }
    });
    tz.on_pick({
        let (ui, state) = (ui.clone(), Rc::clone(&state));
        move |id| {
            let Some(ui) = ui.upgrade() else { return };
            state.borrow_mut().touched = true;
            select(&ui, &id);
        }
    });
    tz.on_pick_at({
        let (ui, state) = (ui.clone(), Rc::clone(&state));
        move |x, y| {
            let Some(ui) = ui.upgrade() else { return };
            let mut state = state.borrow_mut();
            let Some(id) = nearest(&state.on_map, x.into(), y.into()).map(|z| z.id.clone()) else {
                return;
            };
            state.touched = true;
            select(&ui, &id);
        }
    });
    tz.on_save({
        let ui = ui.clone();
        move || {
            let Some(ui) = ui.upgrade() else { return };
            let id = ui.global::<Tz<'_>>().get_current().id;
            crate::SETTINGS.write().timezone = if id == UTC { "UTC".into() } else { id.into() };
        }
    });
    // keeps the clock under the map current
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_secs(15), {
        let ui = ui.clone();
        move || {
            let Some(ui) = ui.upgrade() else { return };
            let id = ui.global::<Tz<'_>>().get_current().id;
            ui.global::<Tz<'_>>()
                .set_current_time(local_time(&id, &locale(&ui)).to_shared_string());
        }
    });
    std::mem::forget(timer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_coords() {
        let (lat, lon) = parse_coords("+4230+00131").unwrap();
        assert!((lat - 42.5).abs() < 1e-9 && (lon - (1. + 31. / 60.)).abs() < 1e-9);
        let (lat, lon) = parse_coords("-345039-0582811").unwrap();
        assert!((lat + 34.844_166).abs() < 1e-4 && (lon + 58.469_722).abs() < 1e-4);
    }

    #[test]
    fn parses_tab() {
        let zones = parse_tab(
            "# comment\nAD\t+4230+00131\tEurope/Andorra\nAE,OM\t+2518+05518\tAsia/Dubai\tCrozet\n",
        );
        assert_eq!(zones.len(), 2);
        assert_eq!(zones[1].id, "Asia/Dubai");
        assert_eq!(zones[1].countries, ["AE", "OM"]);
        assert_eq!(city("America/Argentina/Buenos_Aires"), "Buenos Aires");
    }

    #[test]
    fn nearest_zone_on_the_map() {
        let zones = parse_tab(
            "US\t+404251-0740023\tAmerica/New_York\nUS\t+394421-1045903\tAmerica/Denver\nGB\t+513030-0000731\tEurope/London\nNZ\t-3652+17446\tPacific/Auckland\n",
        );
        let at = |lon: f64, lat: f64| {
            let x = (lon + 180.) / 360.;
            let y = (MAP_NORTH - lat) / (MAP_NORTH - MAP_SOUTH);
            nearest(&zones, x, y).unwrap().id.as_str()
        };
        assert_eq!(at(-100., 40.), "America/Denver");
        assert_eq!(at(-2., 52.), "Europe/London");
        // across the date line from Auckland
        assert_eq!(at(-178., -38.), "Pacific/Auckland");
    }

    #[test]
    fn localized_names_and_times() {
        assert_eq!(Regions::new("en-US").name("US"), "United States");
        assert_eq!(Regions::new("de-DE").name("DE"), "Deutschland");
        let us = local_time("America/Denver", "en-US");
        assert!(us.ends_with("AM") || us.ends_with("PM"), "{us}");
        let de = local_time("Europe/Berlin", "de-DE");
        assert!(de.len() == 5 && de.as_bytes()[2] == b':', "{de}");
        // the real table, if this system has one
        if let Ok(tab) = std::fs::read_to_string(ZONE_TAB) {
            let zones = parse_tab(&tab);
            assert!(zones.len() > 300);
            assert!(
                zones.iter().all(|z| jiff::tz::TimeZone::get(&z.id).is_ok()),
                "zones jiff can't find"
            );
        }
    }

    #[test]
    fn one_dot_per_country_and_rules() {
        let zones = parse_tab(
            "US\t+404251-0740023\tAmerica/New_York\nUS\t+415100-0873900\tAmerica/Chicago\nUS\t+394606-0860929\tAmerica/Indiana/Indianapolis\nUS\t+411745-0863730\tAmerica/Indiana/Knox\nUS\t+332654-1120424\tAmerica/Phoenix\nUS\t+394421-1045903\tAmerica/Denver\n",
        );
        let ids = map_zones(&zones).into_iter().map(|z| z.id).collect::<Vec<_>>();
        // Phoenix has no daylight saving, so it isn't Denver's
        assert_eq!(
            ids,
            ["America/New_York", "America/Chicago", "America/Phoenix", "America/Denver"]
        );
        if let Ok(tab) = std::fs::read_to_string(ZONE_TAB) {
            let zones = parse_tab(&tab);
            let us = map_zones(&zones).into_iter().filter(|z| z.countries[0] == "US").count();
            assert!(us <= 9, "{us} US zones on the map");
        }
    }

    #[test]
    fn offsets() {
        assert_eq!(offset_text("Asia/Kolkata"), "UTC+5:30");
        assert_eq!(offset_text("Etc/UTC"), "UTC");
        assert_eq!(offset_text("Not/AZone"), "UTC");
    }

    #[test]
    fn suggests_from_language_country() {
        let zones =
            parse_tab("JP\t+353916+1394441\tAsia/Tokyo\nDE,DK\t+5230+01322\tEurope/Berlin\n");
        // the system zone wins if it's one of these, which it isn't on a build machine in Denver
        if jiff::tz::TimeZone::system().iana_name().is_none_or(|n| !zones.iter().any(|z| z.id == n))
        {
            assert_eq!(suggest(&zones, "de-DE").as_deref(), Some("Europe/Berlin"));
            assert_eq!(suggest(&zones, "fr"), None);
        }
    }
}
