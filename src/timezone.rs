use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, Model};

use crate::ui::{AppWindow, Lang, TimeZone, Tz};

const ZONE1970_TAB: &str = "/usr/share/zoneinfo/zone1970.tab";
const COUNTRY_TAB: &str = "/usr/share/zoneinfo/zone.tab";
const TZDATA_ZI: &str = "/usr/share/zoneinfo/tzdata.zi";
const MAP_NORTH: f64 = 84.;
const MAP_SOUTH: f64 = -58.;
const UTC: &str = "UTC";

#[derive(Clone, Debug, PartialEq)]
struct Zone {
    id: String,
    countries: Vec<String>,
    lat: f64,
    lon: f64,
    from_zone1970: bool,
}

impl Zone {
    fn main_country(&self) -> Option<&String> {
        self.countries.first()
    }

    fn within_map(&self) -> bool {
        (MAP_SOUTH..=MAP_NORTH).contains(&self.lat)
    }
}

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

fn parse_iso6709_coords(s: &str) -> Option<(f64, f64)> {
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
            let (lat, lon) = parse_iso6709_coords(cols.next()?)?;
            let id = cols.next()?.to_owned();
            Some(Zone { id, countries, lat, lon, from_zone1970: true })
        })
        .collect()
}

fn merge_tabs(mut zones: Vec<Zone>, country_tab_zones: Vec<Zone>) -> Vec<Zone> {
    for zone in country_tab_zones {
        if let Some(z) = zones.iter_mut().find(|z| z.id == zone.id) {
            for c in zone.countries {
                if !z.countries.contains(&c) {
                    z.countries.push(c);
                }
            }
        } else {
            zones.push(Zone { from_zone1970: false, ..zone });
        }
    }
    zones
}

fn parse_zi_links(zi: &str) -> std::collections::HashMap<String, String> {
    zi.lines()
        .filter_map(|line| {
            let mut cols = line.strip_prefix("L ")?.split_whitespace();
            let target = cols.next()?;
            Some((cols.next()?.to_owned(), target.to_owned()))
        })
        .collect()
}

fn city(id: &str) -> String {
    id.rsplit('/').next().unwrap_or(id).replace('_', " ")
}

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

fn icu_locale(locale: &str) -> icu::locale::Locale {
    icu::locale::Locale::try_from_str(locale).unwrap_or(icu::locale::locale!("en-US"))
}

fn local_time(id: &str, locale: &str) -> String {
    use icu::datetime::{NoCalendarFormatter, fieldsets::T};
    let Ok(tz) = jiff::tz::TimeZone::get(id) else { return String::new() };
    let now = jiff::Timestamp::now().to_zoned(tz).time();
    let fmt = NoCalendarFormatter::try_new((&icu_locale(locale)).into(), T::hm()).ok();
    let time =
        icu::time::Time::try_new(now.hour().unsigned_abs(), now.minute().unsigned_abs(), 0, 0).ok();
    match fmt.zip(time) {
        Some((fmt, time)) => fmt.format(&time).to_string(),
        None => now.strftime("%H:%M").to_string(),
    }
}

struct Regions(Option<icu::experimental::displaynames::multi::RegionDisplayNames>);

impl Regions {
    fn new(locale: &str) -> Self {
        use icu::experimental::displaynames::{DisplayNamesOptions, multi::RegionDisplayNames};
        let names = RegionDisplayNames::try_new(
            (&icu_locale(locale)).into(),
            DisplayNamesOptions::default(),
        );
        Self(names.ok())
    }

    fn name(&self, code: &str) -> String {
        let region = icu::locale::subtags::Region::try_from_str(code).ok();
        self.0.as_ref().zip(region).and_then(|(n, r)| n.of(r)).unwrap_or(code).to_owned()
    }
}

fn map_spot(lat: f64, lon: f64) -> (f64, f64) {
    ((lon + 180.) / 360., (MAP_NORTH - lat) / (MAP_NORTH - MAP_SOUTH))
}

fn entry(zone: &Zone, regions: &Regions) -> TimeZone {
    #[expect(clippy::cast_possible_truncation, reason = "a position on screen")]
    let (x, y) = if zone.within_map() {
        let (x, y) = map_spot(zone.lat, zone.lon);
        (x as f32, y as f32)
    } else {
        (-1., -1.)
    };
    TimeZone {
        id: zone.id.as_str().into(),
        city: city(&zone.id).into(),
        region: zone.main_country().map_or_default(|c| regions.name(c)).into(),
        offset: offset_text(&zone.id).into(),
        x,
        y,
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

fn seasonal_offsets(id: &str) -> Option<(i32, i32)> {
    let tz = jiff::tz::TimeZone::get(id).ok()?;
    let year = jiff::Zoned::now().year();
    let at = |month| {
        let date = jiff::civil::date(year, month, 15).to_zoned(tz.clone()).ok()?;
        Some(date.offset().seconds())
    };
    Some((at(1)?, at(7)?))
}

fn one_zone_per_country_and_clocks(zones: &[Zone]) -> Vec<Zone> {
    let mut seen = std::collections::HashSet::new();
    zones
        .iter()
        .filter(|z| z.within_map())
        .filter(|z| seen.insert((z.main_country().cloned(), seasonal_offsets(&z.id))))
        .cloned()
        .collect()
}

const ZONE1970_PREFERENCE_DEGREES: f64 = 0.5;

fn distance_degrees(zone: &Zone, lat: f64, lon: f64) -> f64 {
    let dlon = (zone.lon - lon).abs();
    let dlon = dlon.min(360. - dlon) * lat.to_radians().cos();
    dlon.mul_add(dlon, (zone.lat - lat).powi(2)).sqrt()
}

fn nearest(zones: &[Zone], x_fraction: f64, y_fraction: f64) -> Option<&Zone> {
    let lon = x_fraction.mul_add(360., -180.);
    let lat = y_fraction.mul_add(-(MAP_NORTH - MAP_SOUTH), MAP_NORTH);
    let dist = |z: &Zone| distance_degrees(z, lat, lon);
    let on_map = || zones.iter().filter(|z| z.within_map());
    let best = on_map().min_by(|a, b| dist(a).total_cmp(&dist(b)))?;
    if best.from_zone1970 {
        return Some(best);
    }
    let limit = dist(best) + ZONE1970_PREFERENCE_DEGREES;
    let clocks = seasonal_offsets(&best.id);
    let same_clocks = on_map()
        .filter(|z| z.from_zone1970 && dist(z) <= limit && seasonal_offsets(&z.id) == clocks)
        .min_by(|a, b| dist(a).total_cmp(&dist(b)));
    Some(same_clocks.unwrap_or(best))
}

fn system_zone(
    zones: &mut Vec<Zone>,
    links: &std::collections::HashMap<String, String>,
    name: &str,
) -> Option<String> {
    let current = links.get(name).map_or(name, String::as_str);
    if matches!(current, "Etc/UTC" | "Etc/GMT" | UTC) {
        return None;
    }
    if let Some(z) = [name, current].iter().find_map(|id| zones.iter().find(|z| z.id == *id)) {
        return Some(z.id.clone());
    }
    let id = [current, name].into_iter().find(|id| jiff::tz::TimeZone::get(id).is_ok())?;
    zones.push(Zone {
        id: id.to_owned(),
        countries: vec![],
        lat: f64::NAN,
        lon: f64::NAN,
        from_zone1970: true,
    });
    Some(id.to_owned())
}

fn suggest(zones: &[Zone], system: Option<&str>, locale: &str) -> Option<String> {
    if let Some(name) = system {
        return Some(name.to_owned());
    }
    let locale = icu::locale::Locale::try_from_str(locale).ok()?;
    let region = locale.id.region?;
    let region = region.as_str();
    zones
        .iter()
        .find(|z| z.main_country().is_some_and(|c| c == region))
        .or_else(|| zones.iter().find(|z| z.countries.iter().any(|c| c == region)))
        .map(|z| z.id.clone())
}

fn offset_matches(offset: &str, query: &str) -> bool {
    let offset = offset.to_lowercase().replace('−', "-");
    let query = query.replace('−', "-");
    let ends_in_digit = query.ends_with(|c: char| c.is_ascii_digit());
    let is_whole_or_hours = |offset: &str| {
        offset
            .strip_prefix(query.as_str())
            .is_some_and(|rest| rest.is_empty() || (ends_in_digit && rest.starts_with(':')))
    };
    if query.starts_with(['+', '-']) {
        return offset.strip_prefix("utc").is_some_and(is_whole_or_hours);
    }
    query.starts_with("utc") && is_whole_or_hours(&offset)
}

fn search_text(tz: &TimeZone, zone: Option<&Zone>, regions: &Regions) -> String {
    let countries = zone.into_iter().flat_map(|z| &z.countries).map(|c| regions.name(c));
    let text = [tz.city.to_string(), tz.id.to_string()]
        .into_iter()
        .chain(countries)
        .collect::<Vec<_>>()
        .join("\n");
    crate::l10n::fold_search(&text).replace('−', "-")
}

struct State {
    zones: Vec<Zone>,
    on_map: Vec<Zone>,
    system: Option<String>,
    user_picked: bool,
    guess_from_language: bool,
    query: String,
    entries: Vec<(TimeZone, String)>,
    entries_locale: String,
}

fn locale(ui: &AppWindow) -> String {
    crate::l10n::icu_tag(&ui.global::<Lang<'_>>().get_current_language().locale).to_owned()
}

fn show_results(ui: &AppWindow, state: &State) {
    let query = &state.query;
    let mut results = (state.entries.iter())
        .filter(|(e, text)| text.contains(query.as_str()) || offset_matches(&e.offset, query))
        .map(|(e, _)| e.clone())
        .collect::<Vec<_>>();
    let is_exact_match = |e: &TimeZone| {
        !query.is_empty()
            && (crate::l10n::fold_search(&e.city) == *query || e.id.to_lowercase() == *query)
    };
    results.sort_by(|a, b| {
        is_exact_match(b).cmp(&is_exact_match(a)).then_with(|| a.city.cmp(&b.city))
    });
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
    state.entries = (entries.into_iter().zip(state.zones.iter().map(Some)))
        .chain([(utc_entry(), None)])
        .map(|(e, z)| {
            let text = search_text(&e, z, &regions);
            (e, text)
        })
        .collect();
    show_results(ui, state);
    let id = if state.user_picked {
        tz.get_current().id.to_string()
    } else {
        let suggested = if state.guess_from_language {
            suggest(&state.zones, state.system.as_deref(), &lang)
        } else {
            state.system.clone()
        };
        suggested.unwrap_or_else(|| UTC.to_owned())
    };
    state.entries_locale = lang;
    select(ui, &id);
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| {
        tracing::error!(?e, "cannot read {path}");
        String::new()
    })
}

pub fn initialize_ui(ui: &slint::Weak<AppWindow>, tz: &Tz<'_>) {
    let mut zones = merge_tabs(parse_tab(&read(ZONE1970_TAB)), parse_tab(&read(COUNTRY_TAB)));
    let on_map = one_zone_per_country_and_clocks(&zones);
    let links = parse_zi_links(&read(TZDATA_ZI));
    let system_tz = jiff::tz::TimeZone::system();
    let system = system_tz.iana_name().and_then(|name| system_zone(&mut zones, &links, name));
    tracing::debug!(all = zones.len(), on_map = on_map.len(), ?system, "Loaded time zones");
    let state = Rc::new(RefCell::new(State {
        zones,
        on_map,
        system,
        user_picked: false,
        guess_from_language: crate::cfg::shows_page(crate::cfg::SkipPage::Timezone),
        query: String::new(),
        entries: vec![],
        entries_locale: String::new(),
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
            state.query = crate::l10n::fold_search(query.trim());
            if state.entries_locale == locale(&ui) {
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
            state.borrow_mut().user_picked = true;
            select(&ui, &id);
        }
    });
    tz.on_pick_at({
        let (ui, state) = (ui.clone(), Rc::clone(&state));
        move |x, y| {
            let Some(ui) = ui.upgrade() else { return };
            let mut state = state.borrow_mut();
            let Some(id) = nearest(&state.zones, x.into(), y.into()).map(|z| z.id.clone()) else {
                return;
            };
            state.user_picked = true;
            select(&ui, &id);
        }
    });
    tz.on_save({
        let (ui, state) = (ui.clone(), Rc::clone(&state));
        move || {
            let Some(ui) = ui.upgrade() else { return };
            let id = ui.global::<Tz<'_>>().get_current().id;
            let keep_system_zone = !state.borrow().user_picked && id == UTC;
            crate::SETTINGS.write().timezone =
                if keep_system_zone { String::new() } else { id.into() };
        }
    });
    tick_clock(ui.clone());
}

const MINUTE_MS: i64 = 60_000;
const MINUTE_TURN_MARGIN_MS: i64 = 50;

fn tick_clock(ui: slint::Weak<AppWindow>) {
    let into_minute = jiff::Timestamp::now().as_millisecond().rem_euclid(MINUTE_MS);
    let wait = (MINUTE_MS + MINUTE_TURN_MARGIN_MS - into_minute).unsigned_abs();
    slint::Timer::single_shot(std::time::Duration::from_millis(wait), move || {
        let Some(app) = ui.upgrade() else { return };
        let tz = app.global::<Tz<'_>>();
        tz.set_current_time(local_time(&tz.get_current().id, &locale(&app)).into());
        tick_clock(ui);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_coords() {
        let (lat, lon) = parse_iso6709_coords("+4230+00131").unwrap();
        assert!((lat - 42.5).abs() < 1e-9 && (lon - (1. + 31. / 60.)).abs() < 1e-9);
        let (lat, lon) = parse_iso6709_coords("-345039-0582811").unwrap();
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
            let (x, y) = map_spot(lat, lon);
            nearest(&zones, x, y).unwrap().id.as_str()
        };
        assert_eq!(at(-100., 40.), "America/Denver");
        assert_eq!(at(-2., 52.), "Europe/London");
        assert_eq!(at(-178., -38.), "Pacific/Auckland");
    }

    fn real_zones() -> Option<Vec<Zone>> {
        let main = std::fs::read_to_string(ZONE1970_TAB).ok()?;
        let more = std::fs::read_to_string(COUNTRY_TAB).ok()?;
        Some(merge_tabs(parse_tab(&main), parse_tab(&more)))
    }

    #[test]
    fn nearest_prefers_main_zones() {
        let zones = merge_tabs(
            parse_tab("IT,SM,VA\t+4154+01229\tEurope/Rome\nAT\t+4813+01620\tEurope/Vienna\n"),
            parse_tab("VA\t+415408+0122711\tEurope/Vatican\nSM\t+4355+01228\tEurope/San_Marino\n"),
        );
        let at = |id: &str| {
            let z = zones.iter().find(|z| z.id == id).unwrap();
            let (x, y) = map_spot(z.lat, z.lon);
            nearest(&zones, x, y).unwrap().id.clone()
        };
        assert_eq!(at("Europe/Vatican"), "Europe/Rome");
        assert_eq!(at("Europe/San_Marino"), "Europe/San_Marino");
        assert_eq!(at("Europe/Rome"), "Europe/Rome");
    }

    #[test]
    fn real_tables_pick_each_zone_by_its_coords() {
        let Some(zones) = real_zones() else { return };
        let on_map = || zones.iter().filter(|z| z.within_map());
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
            let (x, y) = map_spot(zone.lat, zone.lon);
            let picked = nearest(&zones, x, y).unwrap();
            assert_eq!(
                seasonal_offsets(&picked.id),
                seasonal_offsets(&zone.id),
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
        if let Ok(tab) = std::fs::read_to_string(ZONE1970_TAB) {
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
        let ids =
            one_zone_per_country_and_clocks(&zones).into_iter().map(|z| z.id).collect::<Vec<_>>();
        assert_eq!(
            ids,
            ["America/New_York", "America/Chicago", "America/Phoenix", "America/Denver"]
        );
        if let Ok(tab) = std::fs::read_to_string(ZONE1970_TAB) {
            let zones = parse_tab(&tab);
            let us = one_zone_per_country_and_clocks(&zones)
                .into_iter()
                .filter(|z| z.countries[0] == "US")
                .count();
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
        assert_eq!(suggest(&zones, None, "nl-NL").as_deref(), Some("Europe/Amsterdam"));
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
        let links = parse_zi_links(
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
        assert_eq!(system_zone(&mut zones, &links, "Etc/GMT-3").as_deref(), Some("Etc/GMT-3"));
        assert_eq!(zones.len(), 2);
        assert!(entry(&zones[1], &Regions::new("en-US")).x < 0.);
    }

    #[test]
    fn real_tables_find_every_country_zone() {
        let Some(zones) = real_zones() else { return };
        for id in ["Europe/Amsterdam", "Europe/Oslo", "Europe/Stockholm", "Europe/Copenhagen"] {
            assert!(zones.iter().any(|z| z.id == id), "{id} missing");
        }
        assert_eq!(suggest(&zones, None, "nl-NL").as_deref(), Some("Europe/Amsterdam"));
        assert_eq!(suggest(&zones, None, "nb-NO").as_deref(), Some("Europe/Oslo"));
        if let Ok(zi) = std::fs::read_to_string(TZDATA_ZI) {
            let mut zones = zones;
            let links = parse_zi_links(&zi);
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
