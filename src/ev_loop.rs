use crate::common::{prepare_js_str, BusConfig, State, UEvent};
use eva_common::err_logger;
use log::{error, info};
use tao::{
    event::{ElementState, Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::KeyCode,
};
use wry::WebView;

err_logger!();

#[allow(clippy::too_many_lines)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
#[allow(deprecated)]
pub fn run(
    event_loop: EventLoop<UEvent>,
    webview: WebView,
    debug: bool,
    bus_config: Option<BusConfig>,
) {
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(ev) => match ev {
                UEvent::GetLocation(resp) => {
                    webview
                        .evaluate_script_with_callback("window.location.href", move |res| {
                            let mut location = None;
                            if let Ok(val) = serde_json::from_str::<String>(&res) {
                                location = Some(val);
                            }
                            let _r = resp.send_blocking(location);
                        })
                        .log_ef();
                }
                UEvent::GetState(resp) => {
                    webview
                        .evaluate_script_with_callback(
                            r"{
                        let result = 0;
                        if (window.$eva.api_token) {
                            result = 2;
                        } else if (window.$eva.hmi.login) {
                            result = 1;
                        }
                        result
                        }",
                            move |res| {
                                let mut state = State::Preparing;
                                if let Ok(val) = serde_json::from_str::<u8>(&res) {
                                    state = val.into();
                                }
                                let _r = resp.send_blocking(state);
                            },
                        )
                        .log_ef();
                }
                UEvent::Login(login, password) => {
                    info!("login requested ({})", login);
                    webview
                        .evaluate_script(&format!(
                            r"$eva.hmi.login('{}', '{}')",
                            prepare_js_str(&login),
                            prepare_js_str(&password)
                        ))
                        .log_ef();
                }
                UEvent::Alert(text, level, timeout) => {
                    let level_str = level.to_string();
                    info!("sending alert ({})", level_str);
                    webview
                        .evaluate_script(&format!(
                            r"$eva.hmi.display_alert('{}', '{}', {})",
                            prepare_js_str(&text),
                            prepare_js_str(&level_str),
                            timeout
                        ))
                        .log_ef();
                }
                UEvent::Logout => {
                    info!("logout requested");
                    webview.evaluate_script("$eva.hmi.logout()").log_ef();
                }
                UEvent::Eval(script) => {
                    info!("eval requested");
                    webview.evaluate_script(&script).log_ef();
                }
                UEvent::Reload => {
                    info!("reload requested");
                    crate::set_stopped();
                    *control_flow = ControlFlow::Exit;
                }
                UEvent::Zoom(level) => {
                    info!("zoom to {} requested", level);
                    if let Err(e) = webview.zoom(level) {
                        error!("zoom error: {}", e);
                    }
                }
                UEvent::Navigate(n_url) => {
                    let url = n_url
                        .as_ref()
                        .unwrap_or_else(|| crate::HOME_URL.get().unwrap());
                    info!("navigate to {} requested", url);
                    webview
                        .evaluate_script(&format!(
                            r#"document.location = "{}""#,
                            prepare_js_str(url)
                        ))
                        .log_ef();
                }
                UEvent::OpenDevTools => {
                    webview.open_devtools();
                }
                UEvent::CloseDevTools => {
                    webview.close_devtools();
                }
            },
            Event::NewEvents(StartCause::Init) => info!("ready"),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                info!("window closed, exiting");
                crate::set_stopped();
                if let Some(ref bus) = bus_config {
                    if bus.is_unix_sock() {
                        let _ = std::fs::remove_file(bus.path());
                    }
                }
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event, .. },
                ..
            } => {
                if debug && event.state == ElementState::Released {
                    #[allow(clippy::single_match)]
                    match event.physical_key {
                        KeyCode::F12 => {
                            if webview.is_devtools_open() {
                                webview.close_devtools();
                            } else {
                                webview.open_devtools();
                            }
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }
    });
}
