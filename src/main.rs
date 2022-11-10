#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg_attr(debug_assertions, windows_subsystem = "console")]
#![warn(clippy::all)]

mod base;
mod rpc;
mod util;
#[cfg(target_os = "windows")]
mod windows;
mod ws;
//use rand::Rng;
use anyhow::Result;
use clap::{Arg, ArgMatches};
use image::GenericImageView;
use rust_embed::RustEmbed;
use serde_json::json;
use std::time;
use wry::{
    application::{
        dpi::LogicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::{Icon, WindowBuilder},
    },
    http::{self, status::StatusCode},
    webview::WebViewBuilder,
};

#[derive(RustEmbed)]
#[folder = "dist/"]
struct Asset;

fn parse_args() -> ArgMatches {
    let app = clap::App::new("GamePerf")
        .version(env!("CARGO_PKG_VERSION"))
        .author("nzcv")
        .about("GamePerf"); 
    app.get_matches()
}

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Install WebView2
        let should_install_webview2 = std::panic::catch_unwind(|| {
            wry::webview::webview_version().expect("Unable to get webview2 version")
        })
        .is_err();
        if should_install_webview2 {
            if let Err(err) = windows::install_webview2().await {
                anyhow::bail!(err)
            }
        }
    }
    let args = parse_args();
    // let server = ws::AwesomeRpc::new(vec!["tse://localhost", "ws://localhost", "http://localhost:*"]);
    // server.start();
    util::init_debug_logger();
    let event_loop = EventLoop::<rpc::Event>::with_user_event();
    let window = WindowBuilder::new()
        .with_title(format!("Trilogy Save Editor - v{} by Karlitos", env!("CARGO_PKG_VERSION")))
        .with_window_icon(load_icon())
        .with_min_inner_size(LogicalSize::new(600, 300))
        .with_inner_size(LogicalSize::new(1000, 700))
        .with_visible(false)
        .with_resizable(false)
        .with_decorations(false)
        .build(&event_loop)?;

    let mut last_maximized_state = window.is_maximized();

    let proxy = event_loop.create_proxy();
    let (tx, rx) = std::sync::mpsc::channel();

    let ipcproxy = proxy.clone();
    let webview = WebViewBuilder::new(window)?
        //.with_initialization_script(&server.initialization_script())
        .with_initialization_script(include_str!("init.js"))
        .with_rpc_handler(move |window, req| {
            rpc::rpc_handler(
                req,
                rpc::RpcUtils { window, event_proxy: &proxy, args: &args, tx: &tx },
            )
        })
        .with_custom_protocol(String::from("tse"), protocol)
        .with_url("tse://localhost/")?
        .build()?;

    #[allow(unused_variables)]
    let server_thread = std::thread::spawn(move || {
        // thread code
        // let _ = webview.evaluate_script("console.log('hello')");
        let mut cur_status = "idle";
        let mut package_name: String = "".into();
        loop {
            if let Ok(msg) = rx.try_recv() {
                match msg {
                    base::ChannelMsg::StartCapture(name) => {
                        cur_status = "runing";
                        package_name = name;
                    }
                    base::ChannelMsg::StopCapture => {
                        cur_status = "idle";
                    }
                }
            }

            match cur_status {
                "idle" => {
                    std::thread::sleep(time::Duration::from_millis(200));
                }
                "runing" => {
                    if !package_name.is_empty() {
                        let pss = util::dump_pss(&package_name);
                        if let Ok(pss) = pss {
                            // let mut rng = rand::thread_rng();
                            // let pss = rng.gen_range(0..20);
                            let _ = ipcproxy
                                .send_event(rpc::Event::BoardCastToJs(json!({ "msg": pss })));
                        }
                    }
                    std::thread::sleep(time::Duration::from_secs(1));
                }
                &_ => {
                    todo!()
                }
            }
        }
    });

    // block on main thread
    let proxy = event_loop.create_proxy();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(_) => {
                    let _ = webview.resize();
                    let is_maximized = webview.window().is_maximized();
                    if is_maximized != last_maximized_state {
                        last_maximized_state = is_maximized;
                        let _ = proxy.send_event(rpc::Event::DispatchCustomEvent(
                            "tse_maximized_state_changed",
                            json!({ "is_maximized": is_maximized }),
                        ));
                    }
                }
                _ => (),
            },
            Event::UserEvent(event) => rpc::event_handler(event, &webview, control_flow),
            Event::LoopDestroyed => {
                // Clear WebView2 Code Cache
                #[cfg(target_os = "windows")]
                windows::clear_code_cache();
            }
            _ => {
                // log::debug!("Loop......");
                // let _ = webview.evaluate_script("console.log('hello')");
            }
        }
    });

    // server_thread.join().unwrap();

    #[allow(unreachable_code)]
    Ok(())
}

fn protocol(request: &http::Request) -> wry::Result<http::Response> {
    let mut path = request.uri().trim_start_matches("tse://localhost/");
    if path.is_empty() {
        path = "index.html"
    }
    log::debug!("{:?}", path);
    let response = http::ResponseBuilder::new();
    match Asset::get(path) {
        Some(asset) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
            response.mimetype(&mime).body(asset.data.into())
        }
        None => response.status(StatusCode::NOT_FOUND).body(vec![]),
    }
}

fn load_icon() -> Option<Icon> {
    let image = image::load_from_memory(include_bytes!("../icon/game.png")).unwrap();
    let (width, height) = image.dimensions();
    let rgba = image.into_rgba8().into_raw();
    Some(Icon::from_rgba(rgba, width, height).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[ctor::ctor]
    fn init() {
        // util::init_debug_logger();
    }

    #[test]
    fn test_dump_pss() -> Result<()> {
        util::init_debug_logger();
        util::dump_pss("com.miHoYo.Yuanshen")?;
        Ok(())
    }
}
