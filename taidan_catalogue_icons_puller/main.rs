use libappstream::glib::GString;
use taidan_catalogue_parser::Choice;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install().unwrap();
    let mut args = std::env::args();
    let bin = args.next().unwrap();
    if args.len() == 0 {
        eprintln!("Usage: {bin} <path/to/yaml> <path/to/another/yaml> ...");
        std::process::exit(1);
    }

    let categories = args
        .map(|p| {
            println!(" Parse: {p}");
            taidan_catalogue_parser::Category::parse_path(std::path::Path::new(&p))
                .map_err(|e| color_eyre::Report::msg(format!("cannot parse {p}")).wrap_err(e))
        })
        .collect::<color_eyre::Result<Vec<_>>>()?;

    let apps: Vec<_> = categories
        .iter()
        .flat_map(|cat| {
            cat.choices
                .iter()
                .map(|app| (cat.name.to_ascii_lowercase(), app.name.to_ascii_lowercase()))
        })
        .collect();

    let num_apps = categories
        .iter()
        .fold(0, |acc, category| acc + category.choices.len());
    println!("Parsed {} categories, {num_apps} apps", categories.len());

    let mut jobs = categories
        .iter()
        .flat_map(|cat| cat.choices.iter())
        .flat_map(to_download_icon)
        .collect::<Vec<_>>();

    // NOTE: fetch flathub
    let client = reqwest::Client::new();
    for x in futures::future::join_all(
        jobs.iter()
            .enumerate()
            .filter_map(|(i, job)| {
                if let Job::Flathub(ref f) = job {
                    Some((i, f))
                } else {
                    None
                }
            })
            .map(|(i, f)| {
                let client = &client;
                async move {
                    let Ok(resp) = client
                        .get(format!(
                            "https://flathub.org/api/v2/appstream/{f}?locale=en"
                        ))
                        .send()
                        .await
                        .inspect_err(|e| println!("E: cannot fetch appstream info: {f}: {e}"))
                    else {
                        return None;
                    };
                    resp.json::<serde_json::Value>()
                        .await
                        .inspect_err(|e| println!("E: cannot deserialize json: {f}: {e}"))
                        .ok()
                        .and_then(|mut r| {
                            let x = r.get_mut("icon").map(std::mem::take);
                            if x.is_none() {
                                println!("E: no icon field: {f}");
                            };
                            x.map(|x| (i, x))
                        })
                }
            }),
    )
    .await
    {
        let Some((i, icon)) = x else { continue };
        let serde_json::Value::String(s) = icon else {
            println!("E: unexpected json value (expected str): {icon}");
            continue;
        };
        jobs[i] = Job::Download(Box::leak(Box::new(s)));
    }

    // NOTE: download
    futures::future::join_all(jobs.iter().enumerate().map(|(i, j)| {
        let apps = &apps;
        let client = &client;
        async move {
            let Job::Download(url) = j else {
                return;
            };
            let Ok(resp) = client
                .get(*url)
                .send()
                .await
                .inspect_err(|e| println!("E: cannot download: {url}: {e}"))
            else {
                return;
            };
            let (ctlg, app) = &apps[i];
            let Ok(mut file) = tokio::fs::File::create(format!("tcip-output/{ctlg}/{app}.ico"))
                .await
                .inspect_err(|e| println!("E: {app}: cannot create request: {e}"))
            else {
                return;
            };
            let Ok(body) = resp
                .bytes()
                .await
                .inspect_err(|e| println!("E: {app}: can't get response bytes: {e}"))
            else {
                return;
            };
            let mut content = std::io::Cursor::new(body);
            _ = tokio::io::copy(&mut content, &mut file)
                .await
                .inspect_err(|e| println!("E: {app}: can't write to file: {e}"));
        }
    }))
    .await;

    // NOTE: copy
    futures::future::join_all(jobs.iter().enumerate().map(|(i, j)| {
        let apps = &apps;
        async move {
            let Job::File(path) = j else {
                return;
            };
            let (ctlg, app) = &apps[i];
            _ = tokio::fs::copy(path, format!("tcip-output/{ctlg}/{app}.ico"))
                .await
                .inspect_err(|e| println!("E: {app}: cannot copy file to tcip-output: {e}"))
        }
    }))
    .await;

    Ok(())
}

fn to_download_icon(Choice { name, icon, .. }: &Choice) -> Option<Job> {
    if let Some((typ, appid)) = icon.split_once(':') {
        match typ {
            "flatpak" => {
                use libappstream::{gio::Cancellable, prelude::*, Pool, PoolFlags};
                let pool = Pool::new();
                pool.add_flags(PoolFlags::LOAD_OS_CATALOG);
                _ = pool.load(Cancellable::NONE);

                let boxed = pool.components_by_id(appid).unwrap();
                let component = boxed.index_safe(0).unwrap();
                if let Some(file) = component.icons().first().and_then(|icon| icon.filename()) {
                    Some(Job::File(file))
                } else {
                    Some(Job::Flathub(appid))
                }
            }
            "icon" => {
                println!("W: {name}: icon:{appid}");
                None
            }
            _ => {
                println!("W: {name}: unknown icon type `{typ}` ({typ}:{appid})");
                None
            }
        }
    } else if icon.starts_with("http://") || icon.starts_with("https://") {
        Some(Job::Download(icon))
    } else {
        // give up?
        println!("W: {name}: unknown icon specifier: {icon}");
        None
    }
}

enum Job<'a> {
    Flathub(&'a str),
    Download(&'a str),
    File(GString),
}
