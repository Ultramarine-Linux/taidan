//! the time zone page: zones from the system tzdata, shown on a map and searchable
use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, Model, ToSharedString};

use crate::ui::{AppWindow, Lang, TimeZone, Tz};

/// zones with their main cities and coordinates, one per region that's had the same clocks since 1970
const ZONE_TAB: &str = "/usr/share/zoneinfo/zone1970.tab";
/// the older table with a zone per country, which has ones `zone1970.tab` merges into another
/// country's (e.g. Europe/Amsterdam into Europe/Brussels) that people still look for
const COUNTRY_TAB: &str = "/usr/share/zoneinfo/zone.tab";
/// the tz database as one file, read for its links from old names (e.g. Asia/Calcutta)
const TZDATA_ZI: &str = "/usr/share/zoneinfo/tzdata.zi";
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
    /// from `zone1970.tab`, not only `zone.tab`, whose extra zones are mostly links to these
    main: bool,
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
    let split = s.get(1..)?.find(['+', '-'])? + 1;
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
            Some(Zone { id, countries, lat, lon, main: true })
        })
        .collect()
}

/// adds the zones of `zone.tab` to those of `zone1970.tab`, and the countries it gives to the
/// ones they share
fn merge_tabs(mut zones: Vec<Zone>, more: Vec<Zone>) -> Vec<Zone> {
    for zone in more {
        if let Some(z) = zones.iter_mut().find(|z| z.id == zone.id) {
            for c in zone.countries {
                if !z.countries.contains(&c) {
                    z.countries.push(c);
                }
            }
        } else {
            zones.push(Zone { main: false, ..zone });
        }
    }
    zones
}

/// old names and what they point to now, from the `L target name` lines of `tzdata.zi`
fn parse_links(zi: &str) -> std::collections::HashMap<String, String> {
    zi.lines()
        .filter_map(|line| {
            let mut cols = line.strip_prefix("L ")?.split_whitespace();
            let target = cols.next()?;
            Some((cols.next()?.to_owned(), target.to_owned()))
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

/// how much farther, in degrees, a `zone1970.tab` zone may be than a nearer `zone.tab` one that
/// keeps the same clocks and still be picked over it, e.g. Rome over the Vatican
const MAIN_SLACK: f64 = 0.5;

/// the zone nearest a spot on the map, given as fractions of its width and height
fn nearest(zones: &[Zone], x: f64, y: f64) -> Option<&Zone> {
    let lon = x.mul_add(360., -180.);
    let lat = y.mul_add(-(MAP_NORTH - MAP_SOUTH), MAP_NORTH);
    let dist = |z: &Zone| {
        // the map wraps around at the date line, and longitude shrinks toward the poles
        let dlon = (z.lon - lon).abs();
        let dlon = dlon.min(360. - dlon) * lat.to_radians().cos();
        dlon.mul_add(dlon, (z.lat - lat).powi(2)).sqrt()
    };
    let on_map = || zones.iter().filter(|z| (MAP_SOUTH..=MAP_NORTH).contains(&z.lat));
    let best = on_map().min_by(|a, b| dist(a).total_cmp(&dist(b)))?;
    if best.main {
        return Some(best);
    }
    // a zone.tab zone is mostly a link to a zone1970.tab one close by, which is the real zone
    let (limit, clocks) = (dist(best) + MAIN_SLACK, rules(&best.id));
    let main = on_map()
        .filter(|z| z.main && dist(z) <= limit && rules(&z.id) == clocks)
        .min_by(|a, b| dist(a).total_cmp(&dist(b)));
    Some(main.unwrap_or(best))
}

/// the system's zone `name` as one of `zones`, under its current name if it's an old one, or
/// added if jiff knows it but the tables don't. None for UTC, which is what a fresh install has
/// and says nothing about where the user is
fn system_zone(
    zones: &mut Vec<Zone>,
    links: &std::collections::HashMap<String, String>,
    name: &str,
) -> Option<String> {
    let current = links.get(name).map_or(name, String::as_str);
    if matches!(current, "Etc/UTC" | "Etc/GMT" | UTC) {
        return None;
    }
    // the name itself if it's listed, e.g. Europe/Amsterdam, which is a link to Europe/Brussels
    if let Some(z) = [name, current].iter().find_map(|id| zones.iter().find(|z| z.id == *id)) {
        return Some(z.id.clone());
    }
    let id = [current, name].into_iter().find(|id| jiff::tz::TimeZone::get(id).is_ok())?;
    // nowhere on the map, but it can be picked in the list
    zones.push(Zone {
        id: id.to_owned(),
        countries: vec![],
        lat: f64::NAN,
        lon: f64::NAN,
        main: true,
    });
    Some(id.to_owned())
}

/// the zone to start with: the system's, else the main one for the language's country, or any
/// that country uses
fn suggest(zones: &[Zone], system: Option<&str>, locale: &str) -> Option<String> {
    if let Some(name) = system {
        return Some(name.to_owned());
    }
    let locale = icu::locale::Locale::try_from_str(locale).ok()?;
    let region = locale.id.region?;
    let region = region.as_str();
    zones
        .iter()
        .find(|z| z.countries.first().is_some_and(|c| c == region))
        .or_else(|| zones.iter().find(|z| z.countries.iter().any(|c| c == region)))
        .map(|z| z.id.clone())
}

/// whether `query` is `offset` (`UTC+5:30`), with or without the `UTC`, or it without the
/// minutes, so `utc+1` finds `UTC+1:30` but not `UTC+10`. Not any part of it, every offset has
/// `utc` in it
fn offset_matches(offset: &str, query: &str) -> bool {
    let offset = offset.to_lowercase().replace('−', "-");
    let query = query.replace('−', "-");
    // the whole offset, or its hours when the minutes are left out
    let ends_in_digit = query.ends_with(|c: char| c.is_ascii_digit());
    let rest = |offset: &str| {
        offset
            .strip_prefix(query.as_str())
            .is_some_and(|rest| rest.is_empty() || (ends_in_digit && rest.starts_with(':')))
    };
    if query.starts_with(['+', '-']) {
        return offset.strip_prefix("utc").is_some_and(rest);
    }
    query.starts_with("utc") && rest(&offset)
}

/// what a zone is found by: its city, every country it's for, and its name
fn search_text(tz: &TimeZone, zone: Option<&Zone>, regions: &Regions) -> String {
    let countries = zone.into_iter().flat_map(|z| &z.countries).map(|c| regions.name(c));
    [tz.city.to_string(), tz.id.to_string()]
        .into_iter()
        .chain(countries)
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase()
        .replace('−', "-")
}

struct State {
    zones: Vec<Zone>,
    /// the ones with a dot on the map, see [`map_zones`]
    on_map: Vec<Zone>,
    /// the system's zone, see [`system_zone`]
    system: Option<String>,
    /// set once the user picks one, so changing the language no longer changes it
    touched: bool,
    /// whether to guess from the language's country when the system has no zone. Off when the
    /// timezone page is skipped, since nothing saves the guess and the system keeps its zone
    guess: bool,
    query: String,
    /// every zone and UTC as shown, with what it's searched by (see [`search_text`]), for the
    /// language in `cached_for`, so typing a search doesn't redo them all
    cache: Vec<(TimeZone, String)>,
    cached_for: String,
}

fn locale(ui: &AppWindow) -> String {
    let locale = ui.global::<Lang<'_>>().get_current_language().locale;
    if locale == "en-owo" { "en-US".to_owned() } else { locale.into() }
}

/// the zones [`State::query`] finds, from [`State::cache`]
fn show_results(ui: &AppWindow, state: &State) {
    let query = &state.query;
    let mut results = (state.cache.iter())
        .filter(|(e, text)| text.contains(query.as_str()) || offset_matches(&e.offset, query))
        .map(|(e, _)| e.clone())
        .collect::<Vec<_>>();
    // one that's exactly what was typed first, e.g. UTC for `utc`, which Enter then picks
    let exact = |e: &TimeZone| {
        !query.is_empty() && (e.city.to_lowercase() == *query || e.id.to_lowercase() == *query)
    };
    results.sort_by(|a, b| exact(b).cmp(&exact(a)).then_with(|| a.city.cmp(&b.city)));
    ui.global::<Tz<'_>>().set_results(slint::ModelRc::new(slint::VecModel::from(results)));
}

fn select(ui: &AppWindow, id: &str) {
    let tz = ui.global::<Tz<'_>>();
    let current = tz.get_all_zones().iter().find(|z| z.id == id).unwrap_or_else(utc_entry);
    tz.set_current_time(local_time(&current.id, &locale(ui)).into());
    tz.set_current(current);
}

fn refresh(ui: &AppWindow, state: &mut State) {
    let lang = locale(ui);
    let regions = Regions::new(&lang);
    let entries = state.zones.iter().map(|z| entry(z, &regions)).collect::<Vec<_>>();
    let tz = ui.global::<Tz<'_>>();
    let dots = state.on_map.iter().map(|z| entry(z, &regions)).collect::<Vec<_>>();
    tz.set_zones(slint::ModelRc::new(slint::VecModel::from(dots)));
    tz.set_all_zones(slint::ModelRc::new(slint::VecModel::from(entries.clone())));
    state.cache = (entries.into_iter().zip(state.zones.iter().map(Some)))
        .chain([(utc_entry(), None)])
        .map(|(e, z)| {
            let text = search_text(&e, z, &regions);
            (e, text)
        })
        .collect();
    state.cached_for = lang;
    show_results(ui, state);
    let id = if state.touched {
        tz.get_current().id.to_string()
    } else {
        let suggested = if state.guess {
            suggest(&state.zones, state.system.as_deref(), &locale(ui))
        } else {
            state.system.clone()
        };
        suggested.unwrap_or_else(|| UTC.to_owned())
    };
    select(ui, &id);
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| {
        tracing::error!(?e, "cannot read {path}");
        String::new()
    })
}

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, tz: &Tz<'_>) {
    let mut zones = merge_tabs(parse_tab(&read(ZONE_TAB)), parse_tab(&read(COUNTRY_TAB)));
    // from all of them, `zone1970.tab` leaves out countries whose clocks have matched another's
    // since 1970 (e.g. Sweden's are Germany's), whose nearest dot could then be in another zone
    let on_map = map_zones(&zones);
    let links = parse_links(&read(TZDATA_ZI));
    let system_tz = jiff::tz::TimeZone::system();
    let system = system_tz.iana_name().and_then(|name| system_zone(&mut zones, &links, name));
    tracing::debug!(all = zones.len(), on_map = on_map.len(), ?system, "Loaded time zones");
    let state = Rc::new(RefCell::new(State {
        zones,
        on_map,
        system,
        touched: false,
        guess: !crate::CFG.skip_pages.contains(&crate::cfg::SkipPage::Timezone),
        query: String::new(),
        cache: vec![],
        cached_for: String::new(),
    }));
    tz.on_refresh({
        let (ui, state) = (ui.clone(), Rc::clone(&state));
        move || {
            if let Some(ui) = ui.upgrade() {
                refresh(&ui, &mut state.borrow_mut());
            }
        }
    });
    tz.on_search({
        let (ui, state) = (ui.clone(), Rc::clone(&state));
        move |query| {
            let Some(ui) = ui.upgrade() else { return };
            let mut state = state.borrow_mut();
            state.query = query.trim().to_lowercase();
            if state.cached_for == locale(&ui) {
                show_results(&ui, &state);
            } else {
                refresh(&ui, &mut state);
            }
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
            // every zone rather than only the dots, which leave out ones that differ only in
            // history and may be nearer
            let Some(id) = nearest(&state.zones, x.into(), y.into()).map(|z| z.id.clone()) else {
                return;
            };
            state.touched = true;
            select(&ui, &id);
        }
    });
    tz.on_save({
        let (ui, state) = (ui.clone(), Rc::clone(&state));
        move || {
            let Some(ui) = ui.upgrade() else { return };
            let id = ui.global::<Tz<'_>>().get_current().id;
            // UTC only because nothing better was known, so the system keeps the zone it has
            let keep = !state.borrow().touched && id == UTC;
            crate::SETTINGS.write().timezone = if keep { String::new() } else { id.into() };
        }
    });
    tick_clock(ui.clone());
}

/// keeps the clock under the map current, updating it just after each minute starts
fn tick_clock(ui: slint::Weak<AppWindow>) {
    let into_minute = jiff::Timestamp::now().as_millisecond().rem_euclid(60_000);
    // a little after, so the minute has surely turned when it's read
    let wait = u64::try_from(60_050 - into_minute).unwrap_or(60_000);
    slint::Timer::single_shot(std::time::Duration::from_millis(wait), move || {
        let Some(app) = ui.upgrade() else { return };
        let id = app.global::<Tz<'_>>().get_current().id;
        app.global::<Tz<'_>>().set_current_time(local_time(&id, &locale(&app)).to_shared_string());
        tick_clock(ui);
    });
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

    /// x and y on the map for a zone's own coordinates
    fn spot(z: &Zone) -> (f64, f64) {
        ((z.lon + 180.) / 360., (MAP_NORTH - z.lat) / (MAP_NORTH - MAP_SOUTH))
    }

    #[test]
    fn nearest_prefers_main_zones() {
        let zones = merge_tabs(
            parse_tab("IT,SM,VA\t+4154+01229\tEurope/Rome\nAT\t+4813+01620\tEurope/Vienna\n"),
            parse_tab("VA\t+415408+0122711\tEurope/Vatican\nSM\t+4355+01228\tEurope/San_Marino\n"),
        );
        let at = |id: &str| {
            let (x, y) = spot(zones.iter().find(|z| z.id == id).unwrap());
            nearest(&zones, x, y).unwrap().id.clone()
        };
        // the Vatican links to Rome, a few hundredths of a degree away
        assert_eq!(at("Europe/Vatican"), "Europe/Rome");
        // two degrees away, too far to be taken for Rome
        assert_eq!(at("Europe/San_Marino"), "Europe/San_Marino");
        assert_eq!(at("Europe/Rome"), "Europe/Rome");
    }

    #[test]
    fn real_tables_pick_each_zone_by_its_coords() {
        let (Ok(main), Ok(more)) =
            (std::fs::read_to_string(ZONE_TAB), std::fs::read_to_string(COUNTRY_TAB))
        else {
            return;
        };
        let zones = merge_tabs(parse_tab(&main), parse_tab(&more));
        let on_map = || zones.iter().filter(|z| (MAP_SOUTH..=MAP_NORTH).contains(&z.lat));
        for id in [
            "Australia/Sydney",
            "Australia/Melbourne",
            "America/Sao_Paulo",
            "America/Indiana/Indianapolis",
            "Europe/Volgograd",
        ] {
            assert!(on_map().any(|z| z.id == id), "{id} not on the map");
        }
        for zone in on_map() {
            let (x, y) = spot(zone);
            let picked = nearest(&zones, x, y).unwrap();
            assert_eq!(
                rules(&picked.id),
                rules(&zone.id),
                "clicking {} picks {}, which keeps other clocks",
                zone.id,
                picked.id
            );
        }
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
        let zones = merge_tabs(
            parse_tab(
                "JP\t+353916+1394441\tAsia/Tokyo\nDE,DK\t+5230+01322\tEurope/Berlin\nBE,LU,NL\t+5050+00420\tEurope/Brussels\n",
            ),
            parse_tab("NL\t+5222+00454\tEurope/Amsterdam\nBE\t+5050+00420\tEurope/Brussels\n"),
        );
        assert_eq!(suggest(&zones, None, "de-DE").as_deref(), Some("Europe/Berlin"));
        // not the first country of any zone in zone1970.tab, but its own in zone.tab
        assert_eq!(suggest(&zones, None, "nl-NL").as_deref(), Some("Europe/Amsterdam"));
        // only a zone shared with another country
        assert_eq!(suggest(&zones, None, "da-DK").as_deref(), Some("Europe/Berlin"));
        assert_eq!(suggest(&zones, None, "fr"), None);
        assert_eq!(suggest(&zones, Some("Asia/Tokyo"), "de-DE").as_deref(), Some("Asia/Tokyo"));
    }

    #[test]
    fn merges_tabs() {
        let zones = merge_tabs(
            parse_tab("BE,LU,NL\t+5050+00420\tEurope/Brussels\n"),
            parse_tab(
                "NL\t+5222+00454\tEurope/Amsterdam\nBE\t+5050+00420\tEurope/Brussels\nLU\t+4936+00609\tEurope/Luxembourg\n",
            ),
        );
        let ids = zones.iter().map(|z| z.id.as_str()).collect::<Vec<_>>();
        assert_eq!(ids, ["Europe/Brussels", "Europe/Amsterdam", "Europe/Luxembourg"]);
        assert_eq!(zones[0].countries, ["BE", "LU", "NL"]);
        let regions = Regions::new("en-US");
        let brussels = entry(&zones[0], &regions);
        let text = search_text(&brussels, Some(&zones[0]), &regions);
        assert!(text.contains("luxembourg") && text.contains("netherlands"), "{text}");
    }

    #[test]
    fn offsets_found_whole() {
        assert!(offset_matches("UTC", "utc"));
        assert!(!offset_matches("UTC+5:30", "utc"));
        assert!(!offset_matches("UTC+5:30", "u"));
        assert!(offset_matches("UTC+5:30", "utc+5"));
        assert!(offset_matches("UTC+5:30", "+5:30"));
        assert!(offset_matches("UTC+1", "utc+1"));
        assert!(!offset_matches("UTC+10", "utc+1"));
        assert!(offset_matches("UTC−3", "utc-3"));
    }

    #[test]
    fn system_zone_old_names_and_utc() {
        let links = parse_links(
            "# comment\nZ Asia/Kolkata 5:53:28 - LMT\nL Asia/Kolkata Asia/Calcutta\nL Etc/UTC UTC\nL Etc/UTC Etc/Universal\n",
        );
        assert_eq!(links["Asia/Calcutta"], "Asia/Kolkata");
        let mut zones = parse_tab("IN\t+2232+08822\tAsia/Kolkata\n");
        assert_eq!(
            system_zone(&mut zones, &links, "Asia/Calcutta").as_deref(),
            Some("Asia/Kolkata")
        );
        assert_eq!(
            system_zone(&mut zones, &links, "Asia/Kolkata").as_deref(),
            Some("Asia/Kolkata")
        );
        assert_eq!(system_zone(&mut zones, &links, "Etc/Universal"), None);
        assert_eq!(system_zone(&mut zones, &links, "UTC"), None);
        assert_eq!(system_zone(&mut zones, &links, "Not/AZone"), None);
        assert_eq!(zones.len(), 1);
        // not in the tables, but jiff knows it, so it's added off the map
        assert_eq!(system_zone(&mut zones, &links, "Etc/GMT-3").as_deref(), Some("Etc/GMT-3"));
        assert_eq!(zones.len(), 2);
        assert!(entry(&zones[1], &Regions::new("en-US")).x < 0.);
    }

    #[test]
    fn real_tables_find_every_country_zone() {
        let (Ok(main), Ok(more)) =
            (std::fs::read_to_string(ZONE_TAB), std::fs::read_to_string(COUNTRY_TAB))
        else {
            return;
        };
        let zones = merge_tabs(parse_tab(&main), parse_tab(&more));
        for id in ["Europe/Amsterdam", "Europe/Oslo", "Europe/Stockholm", "Europe/Copenhagen"] {
            assert!(zones.iter().any(|z| z.id == id), "{id} missing");
        }
        assert_eq!(suggest(&zones, None, "nl-NL").as_deref(), Some("Europe/Amsterdam"));
        assert_eq!(suggest(&zones, None, "nb-NO").as_deref(), Some("Europe/Oslo"));
        if let Ok(zi) = std::fs::read_to_string(TZDATA_ZI) {
            let mut zones = zones;
            let links = parse_links(&zi);
            assert_eq!(
                system_zone(&mut zones, &links, "Asia/Calcutta").as_deref(),
                Some("Asia/Kolkata")
            );
            assert_eq!(
                system_zone(&mut zones, &links, "Europe/Amsterdam").as_deref(),
                Some("Europe/Amsterdam")
            );
        }
    }
}
