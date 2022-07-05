use clap::Parser;
use eva_common::{EResult, Error};
use log::{debug, info};
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::collections::HashSet;
use std::fmt::Write as _;
use std::sync::atomic;
use std::thread;
use wry::{
    application::{event_loop::EventLoop, window::Fullscreen, window::WindowBuilder},
    webview::WebViewBuilder,
};

mod common;
mod eapi;
mod ev_loop;

use common::{BusConfig, PanelInfo, UEvent};

static HOME_URL: OnceCell<String> = OnceCell::new();
static ALLOWED_URLS: OnceCell<HashSet<String>> = OnceCell::new();
static MONITOR: OnceCell<String> = OnceCell::new();
static REBOOT_CMD: OnceCell<String> = OnceCell::new();
static ACTIVE: atomic::AtomicBool = atomic::AtomicBool::new(true);
static DEBUG: atomic::AtomicBool = atomic::AtomicBool::new(false);
const AGENT_NAME: &str = "EvaPanel";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const ARCH: &str = env!("ARCH");
const WEB_ENGINE: &str = "WebKit";

#[inline]
fn set_stopped() {
    ACTIVE.store(false, atomic::Ordering::SeqCst);
}

#[inline]
fn is_active() -> bool {
    ACTIVE.load(atomic::Ordering::SeqCst)
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short = 'c', long = "config", default_value = "~/evapanel.yml")]
    config_path: String,
}

#[inline]
fn default_home_url() -> String {
    "http://eva/ui/".to_owned()
}

#[inline]
fn default_zoom() -> f64 {
    1.0
}

#[inline]
fn default_reboot_cmd() -> String {
    "reboot".to_owned()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(default = "default_home_url")]
    home_url: String,
    #[serde(default = "default_zoom")]
    zoom: f64,
    #[serde(default)]
    engine: common::Engine,
    #[serde(default)]
    allowed_urls: HashSet<String>,
    #[serde(default)]
    show_cursor: bool,
    #[serde(default)]
    debug: bool,
    #[serde(default)]
    sig: Option<String>,
    #[serde(default)]
    bus: Option<BusConfig>,
    commands: Commands,
}

#[derive(Deserialize)]
struct Commands {
    #[serde(default = "default_reboot_cmd")]
    reboot: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            home_url: default_home_url(),
            zoom: default_zoom(),
            engine: <_>::default(),
            allowed_urls: HashSet::new(),
            show_cursor: false,
            debug: false,
            sig: None,
            bus: None,
            commands: <_>::default(),
        }
    }
}

impl Default for Commands {
    fn default() -> Self {
        Self {
            reboot: default_reboot_cmd(),
        }
    }
}

#[inline]
fn url_allowed(url: &str) -> bool {
    for allowed_url in ALLOWED_URLS.get().unwrap() {
        if url.starts_with(allowed_url) {
            return true;
        }
    }
    false
}

#[allow(clippy::too_many_lines)]
fn main() -> EResult<()> {
    let args = Args::parse();
    let (config, used_default) = match std::fs::read(shellexpand::tilde(&args.config_path).as_ref())
    {
        Ok(v) => (
            serde_yaml::from_slice(&v).map_err(|e| {
                Error::invalid_data(format!("Unable to parse {}: {}", args.config_path, e))
            })?,
            false,
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Config::default(), true),
        Err(e) => {
            return Err(Error::io(format!(
                "Unable to open {}: {}",
                args.config_path, e
            )))
        }
    };
    env_logger::Builder::new()
        .target(env_logger::Target::Stdout)
        .filter_level(if config.debug {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .init();
    info!(
        "Using config: {}",
        if used_default {
            "default"
        } else {
            &args.config_path
        }
    );
    let mut user_agent = format!(
        "{} {} {}/{} ({})",
        AGENT_NAME, VERSION, ARCH, config.engine, WEB_ENGINE
    );
    if let Some(sig) = config.sig {
        write!(user_agent, " {}", sig).map_err(Error::failed)?;
    }
    let allow_any = config.allowed_urls.contains("*");
    let mut allowed_urls = config.allowed_urls;
    allowed_urls.insert(config.home_url.clone());
    info!("starting {}", user_agent);
    debug!("home_url: {}", config.home_url);
    debug!("zoom: {}", config.zoom);
    debug!("engine: {}", config.engine);
    debug!(
        "allow urls: {}",
        allowed_urls
            .iter()
            .map(String::as_str)
            .collect::<Vec<&str>>()
            .join(", ")
    );
    debug!("reboot_cmd: {}", config.commands.reboot);
    debug!("user agent: {}", user_agent);
    debug!("allow any: {}", allow_any);
    HOME_URL.set(config.home_url.clone()).unwrap();
    REBOOT_CMD.set(config.commands.reboot).unwrap();
    ALLOWED_URLS.set(allowed_urls).unwrap();
    let event_loop: EventLoop<UEvent> = EventLoop::with_user_event();
    info!("creating HMI window");
    let window = WindowBuilder::new()
        .with_title("EVA ICS Panel")
        .build(&event_loop)
        .map_err(Error::failed)?;
    window.set_cursor_visible(config.show_cursor);
    window.set_fullscreen(Some(Fullscreen::Borderless(None)));
    if let Some(monitor) = window.current_monitor().and_then(|v| v.name()) {
        info!("monitor: {}", monitor);
        MONITOR.set(monitor).unwrap();
    }
    info!("creating Web view");
    let webview = WebViewBuilder::new(window)
        .map_err(Error::failed)?
        .with_user_agent(&user_agent)
        .with_navigation_handler(move |url| allow_any || url_allowed(&url))
        .with_url(&config.home_url)
        .map_err(Error::failed)?
        .with_devtools(config.debug)
        .build()
        .map_err(Error::failed)?;
    DEBUG.store(config.debug, atomic::Ordering::SeqCst);
    webview.zoom(config.zoom);
    info!("starting event loop");
    if let Some(bus) = config.bus {
        let panel_info = PanelInfo {
            home_url: config.home_url,
            agent: AGENT_NAME.to_owned(),
            version: VERSION.to_owned(),
            engine: config.engine,
            arch: ARCH.to_owned(),
            debug: config.debug,
        };
        let api_proxy = event_loop.create_proxy();
        thread::spawn(move || {
            eapi::launch(&bus, api_proxy, panel_info);
        });
    }
    ev_loop::run(event_loop, webview, config.debug);
    Ok(())
}
