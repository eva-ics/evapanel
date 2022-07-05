use eva_common::{EResult, Error};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::future::Future;
use std::time::Duration;
use tokio::sync::oneshot;

const CMD_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Serialize)]
pub struct PanelInfo {
    pub(crate) home_url: String,
    pub(crate) agent: String,
    pub(crate) version: String,
    pub(crate) arch: String,
    pub(crate) engine: Engine,
    pub(crate) debug: bool,
}

#[derive(bmart::tools::EnumStr, Serialize, Copy, Clone)]
#[repr(u8)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Preparing = 0,
    Loaded = 1,
    Active = 2,
    Unknown = 0xff,
}

impl From<u8> for State {
    fn from(code: u8) -> State {
        match code {
            0 => State::Preparing,
            1 => State::Loaded,
            2 => State::Active,
            _ => State::Unknown,
        }
    }
}

#[derive(Serialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct StateInfo<'a> {
    home_url: &'a str,
    current_url: Option<&'a str>,
    agent: &'a str,
    version: &'a str,
    arch: &'a str,
    engine: Engine,
    debug: bool,
    state: State,
}

impl PanelInfo {
    pub fn state_info<'a>(&'a self, state: State, current_url: Option<&'a str>) -> StateInfo<'a> {
        StateInfo {
            home_url: &self.home_url,
            current_url,
            agent: &self.agent,
            version: &self.version,
            arch: &self.arch,
            engine: self.engine,
            debug: self.debug,
            state,
        }
    }
}

#[derive(Deserialize, Serialize, bmart::tools::EnumStr, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    Wasm,
    Js,
}

#[derive(Deserialize, Serialize, bmart::tools::EnumStr, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum BusMode {
    Server,
    Client,
}

impl Default for Engine {
    fn default() -> Self {
        Self::Wasm
    }
}

#[derive(Deserialize)]
pub struct BusConfig {
    mode: BusMode,
    path: String,
}

impl BusConfig {
    #[inline]
    pub fn path(&self) -> &str {
        &self.path
    }
    #[inline]
    pub fn mode(&self) -> BusMode {
        self.mode
    }
}

pub enum UEvent {
    Login(String, String),
    Logout,
    Eval(String),
    Zoom(f64),
    Navigate(Option<String>),
    Alert(String, AlertLevel, u16),
    Reload,
    OpenDevTools,
    CloseDevTools,
    GetState(oneshot::Sender<State>),
    GetLocation(oneshot::Sender<Option<String>>),
}

#[derive(Deserialize, bmart::tools::EnumStr)]
#[serde(rename_all = "lowercase")]
pub enum AlertLevel {
    Info,
    Warning,
}

impl Default for AlertLevel {
    fn default() -> Self {
        Self::Info
    }
}

#[inline]
pub fn prepare_js_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\"', "\\\"")
}

pub fn system_cmd<'a, I, S>(cmd: &'a str, args: I) -> impl Future<Output = EResult<()>> + 'a
where
    I: IntoIterator<Item = S> + 'a,
    S: AsRef<OsStr> + 'a,
{
    system_cmd_x(cmd, args, &[])
}

pub async fn system_cmd_x<I, S>(cmd: &str, args: I, exit_ok: &[i32]) -> EResult<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let res =
        bmart::process::command(cmd, args, CMD_TIMEOUT, bmart::process::Options::default()).await?;
    let code = res.code.unwrap_or(-1);
    if code == 0 || exit_ok.contains(&code) {
        Ok(())
    } else {
        Err(Error::failed(format!(
            "process exit code {}\n{}",
            code,
            res.err.join("\n")
        )))
    }
}
