use crate::common::{system_cmd, system_cmd_x, AlertLevel, BusConfig, BusMode, PanelInfo, UEvent};
use busrt::rpc::{Rpc, RpcClient, RpcError, RpcEvent, RpcHandlers, RpcResult};
use eva_common::payload::{pack, unpack};
use eva_common::Error;
use eva_common::{err_logger, EResult};
use log::{info, warn};
use serde::Deserialize;
use std::sync::atomic;
use std::time::Duration;
use tao::event_loop::EventLoopProxy;

err_logger!();

const DEFAULT_BUS_TIMEOUT: Duration = Duration::from_secs(5);

pub struct Handlers {
    api_proxy: EventLoopProxy<UEvent>,
    info: PanelInfo,
}

#[inline]
fn default_alert_timeout() -> u16 {
    30
}

async fn set_display(monitor: &str, on: bool) -> EResult<()> {
    let args = vec!["--output", monitor, if on { "--auto" } else { "--off" }];
    info!("setting display {} on={}", monitor, on);
    if on {
        system_cmd("xrandr", args).await
    } else {
        // xrandr exit code 1 is ok for off
        system_cmd_x("xrandr", args, &[1]).await
    }
}

#[async_trait::async_trait]
impl RpcHandlers for Handlers {
    #[allow(clippy::too_many_lines)]
    async fn handle_call(&self, event: RpcEvent) -> RpcResult {
        macro_rules! send_event {
            ($ev: expr) => {
                self.api_proxy
                    .send_event($ev)
                    .map_err(|_| RpcError::internal(None))?;
            };
        }
        macro_rules! need_debug {
            () => {
                if !crate::DEBUG.load(atomic::Ordering::Relaxed) {
                    return Err(Error::access("debug mode is not enabled").into());
                }
            };
        }
        let payload = event.payload();
        match event.parse_method()? {
            "login" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct ParamsLogin {
                    login: String,
                    password: String,
                }
                if payload.is_empty() {
                    Err(RpcError::params(None))
                } else {
                    let p: ParamsLogin = unpack(payload)?;
                    send_event!(UEvent::Login(p.login, p.password));
                    Ok(None)
                }
            }
            "logout" => {
                if payload.is_empty() {
                    self.api_proxy
                        .send_event(UEvent::Logout)
                        .map_err(|_| RpcError::internal(None))?;
                    Ok(None)
                } else {
                    Err(RpcError::params(None))
                }
            }
            "test" => {
                if payload.is_empty() {
                    Ok(None)
                } else {
                    Err(RpcError::params(None))
                }
            }
            "info" => {
                if payload.is_empty() {
                    let (tx, rx) = async_channel::bounded(1);
                    send_event!(UEvent::GetState(tx));
                    let state = tokio::time::timeout(Duration::from_secs(1), rx.recv())
                        .await
                        .map_err(|_| Error::timeout())?
                        .map_err(Error::failed)?;
                    let (tx, rx) = async_channel::bounded(1);
                    send_event!(UEvent::GetLocation(tx));
                    let current_url = tokio::time::timeout(Duration::from_secs(1), rx.recv())
                        .await
                        .map_err(|_| Error::timeout())?
                        .map_err(Error::failed)?;
                    Ok(Some(pack(
                        &self.info.state_info(state, current_url.as_deref()),
                    )?))
                } else {
                    Err(RpcError::params(None))
                }
            }
            "alert" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct ParamsAlert {
                    text: String,
                    level: Option<AlertLevel>,
                    timeout: Option<u16>,
                }
                if payload.is_empty() {
                    Err(RpcError::params(None))
                } else {
                    let p: ParamsAlert = unpack(payload)?;
                    send_event!(UEvent::Alert(
                        p.text,
                        p.level.unwrap_or_default(),
                        p.timeout.unwrap_or_else(default_alert_timeout)
                    ));
                    Ok(None)
                }
            }
            "eval" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct ParamsEval {
                    code: String,
                }
                if payload.is_empty() {
                    Err(RpcError::params(None))
                } else {
                    let p: ParamsEval = unpack(payload)?;
                    send_event!(UEvent::Eval(p.code));
                    Ok(None)
                }
            }
            "navigate" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct ParamsNavigate {
                    url: Option<String>,
                }
                if payload.is_empty() {
                    send_event!(UEvent::Navigate(None));
                    Ok(None)
                } else {
                    let p: ParamsNavigate = unpack(payload)?;
                    send_event!(UEvent::Navigate(p.url));
                    Ok(None)
                }
            }
            "display" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct ParamsDisplay {
                    on: Option<bool>,
                    brightness: Option<f32>,
                }
                if payload.is_empty() {
                    Err(RpcError::params(None))
                } else if let Some(monitor) = crate::MONITOR.get() {
                    let p: ParamsDisplay = unpack(payload)?;
                    if let Some(brightness) = p.brightness {
                        if brightness > 100.0 {
                            return Err(Error::invalid_params("brightness > 100%").into());
                        }
                        info!("setting display brightness to {}%", brightness);
                        let br_str = brightness.to_string();
                        let args = vec!["-set", &br_str];
                        system_cmd("xbacklight", args).await?;
                    }
                    if let Some(on) = p.on {
                        if on {
                            set_display(monitor, false).await?;
                            tokio::time::sleep(Duration::from_millis(500)).await;
                            set_display(monitor, true).await?;
                        } else {
                            set_display(monitor, false).await?;
                        }
                    }
                    Ok(None)
                } else {
                    Err(Error::failed("monitor not detected").into())
                }
            }
            "zoom" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct ParamsZoom {
                    level: f64,
                }
                if payload.is_empty() {
                    Err(RpcError::params(None))
                } else {
                    let p: ParamsZoom = unpack(payload)?;
                    send_event!(UEvent::Zoom(p.level));
                    Ok(None)
                }
            }
            "reload" | "stop" => {
                if payload.is_empty() {
                    send_event!(UEvent::Reload);
                    Ok(None)
                } else {
                    Err(RpcError::params(None))
                }
            }
            "reboot" => {
                if payload.is_empty() {
                    let args = vec!["-c", crate::REBOOT_CMD.get().unwrap()];
                    warn!("calling reboot command");
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        system_cmd("sh", args).await.log_ef();
                    });
                    Ok(None)
                } else {
                    Err(RpcError::params(None))
                }
            }
            "dev.open" => {
                if payload.is_empty() {
                    need_debug!();
                    send_event!(UEvent::OpenDevTools);
                    Ok(None)
                } else {
                    Err(RpcError::params(None))
                }
            }
            "dev.close" => {
                if payload.is_empty() {
                    need_debug!();
                    send_event!(UEvent::CloseDevTools);
                    Ok(None)
                } else {
                    Err(RpcError::params(None))
                }
            }
            _ => Err(RpcError::method(None)),
        }
    }
    async fn handle_notification(&self, _event: RpcEvent) {}
    async fn handle_frame(&self, _frame: busrt::Frame) {}
}

pub async fn launch_bus(
    bus: &BusConfig,
    api_proxy: EventLoopProxy<UEvent>,
    panel_info: PanelInfo,
) -> EResult<()> {
    let path = bus.path();
    let handlers = Handlers {
        api_proxy,
        info: panel_info,
    };
    let sleep_step = Duration::from_millis(200);
    match bus.mode() {
        #[cfg(target_os = "linux")]
        BusMode::Server => {
            let mut broker = busrt::broker::Broker::new();
            let server_config = busrt::broker::ServerConfig::new().timeout(DEFAULT_BUS_TIMEOUT);
            if bus.is_unix_sock() {
                broker.spawn_unix_server(path, server_config).await?;
                info!("BUS/RT control UNIX socket: {}", path);
            } else {
                broker.spawn_tcp_server(path, server_config).await?;
                info!("BUS/RT control TCP socket: {}", path);
            }
            let client = broker.register_client(".panel").await.unwrap();
            let _rpc = RpcClient::new(client, handlers);
            while crate::is_active() {
                tokio::time::sleep(sleep_step).await;
            }
        }
        BusMode::Client => {
            let name = format!(
                "eva.panel.{}",
                hostname::get().map_err(Error::failed)?.to_string_lossy()
            );
            let client = busrt::ipc::Client::connect(
                &busrt::ipc::Config::new(path, &name).timeout(DEFAULT_BUS_TIMEOUT),
            )
            .await?;
            info!("connected to BUS/RT broker at {} as {}", path, name);
            let rpc = RpcClient::new(client, handlers);
            while rpc.client().lock().await.is_connected() {
                tokio::time::sleep(sleep_step).await;
            }
            std::process::exit(0);
        }
    }
}

pub fn launch(bus: &BusConfig, api_proxy: EventLoopProxy<UEvent>, panel_info: PanelInfo) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        if launch_bus(bus, api_proxy, panel_info)
            .await
            .log_err()
            .is_err()
        {
            std::process::exit(1);
        }
    });
}
