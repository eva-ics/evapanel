use crate::common::{prepare_js_str, BusConfig, State, UEvent};
use eva_common::err_logger;
use log::info;
use webkit2gtk::WebViewExt;
use wry::webview::WebviewExtUnix;
use wry::{
    application::{
        event::{ElementState, Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        keyboard::KeyCode,
    },
    webview::WebView,
};

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
    let wv = webview.webview();
    let cancellable: Option<&gio::Cancellable> = None;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(ev) => match ev {
                UEvent::GetLocation(resp) => {
                    wv.run_javascript("window.location.href", cancellable, |res| {
                        let mut location = None;
                        if let Ok(result) = res {
                            if let Some(ref ctx) = result.global_context() {
                                if let Some(val) = result.value().and_then(|v| v.to_string(ctx)) {
                                    location = Some(val);
                                }
                            }
                        }
                        let _r = resp.send(location);
                    });
                }
                UEvent::GetState(resp) => {
                    wv.run_javascript(
                        r#"{
                        let result = 0;
                        if (window.$eva.api_token) {
                            result = 2;
                        } else if (window.$eva.hmi.login) {
                            result = 1;
                        }
                        result
                        }"#,
                        cancellable,
                        |res| {
                            let mut state = State::Preparing;
                            if let Ok(result) = res {
                                if let Some(ref ctx) = result.global_context() {
                                    if let Some(val) = result.value().and_then(|v| v.to_number(ctx))
                                    {
                                        state = (val as u8).into();
                                    }
                                }
                            }
                            let _r = resp.send(state);
                        },
                    );
                }
                UEvent::Login(login, password) => {
                    info!("login requested ({})", login);
                    webview
                        .evaluate_script(&format!(
                            r#"$eva.hmi.login('{}', '{}')"#,
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
                            r#"$eva.hmi.display_alert('{}', '{}', {})"#,
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
                    webview.zoom(level);
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
                        KeyCode::F6 => {
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
