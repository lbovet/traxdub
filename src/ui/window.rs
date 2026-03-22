use anyhow::Result;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock, atomic::{AtomicBool, Ordering}};
use tao::event_loop::EventLoopProxy;

use crate::ui::{GridElement, MenuOptionElement, LinkType};

#[derive(Debug, Clone)]
pub enum UserEvent {
    Quit,
}

static PROXY: OnceLock<EventLoopProxy<UserEvent>> = OnceLock::new();
static RUNNING: OnceLock<Arc<AtomicBool>> = OnceLock::new();

pub fn close() -> Result<()> {
    if let Some(proxy) = PROXY.get() {
        proxy.send_event(UserEvent::Quit)
            .map_err(|e| anyhow::anyhow!("Failed to send quit event: {:?}", e))?;
    }
    Ok(())
}

/// Create and run the UI window  
pub fn run(
    message_queue: Arc<Mutex<VecDeque<String>>>,
    focused_grid_element: Arc<Mutex<Option<GridElement>>>,
    focused_menu_option: Arc<Mutex<Option<MenuOptionElement>>>,
) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let _ = RUNNING.set(Arc::clone(&running));
    use wry::{
        dpi::LogicalSize,
        WebViewBuilder,
    };
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
        window::WindowBuilder,
    };

    let event_loop: EventLoop<UserEvent> = EventLoopBuilder::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let _ = PROXY.set(proxy);
    let window = WindowBuilder::new()
        .with_title("TraxDub")
        .with_background_color((0x1a, 0x1a, 0x1a, 0xff).into())
        .with_inner_size(LogicalSize::new(800.0, 600.0))
        .build(&event_loop)?;

    let window_html = include_str!("window.html").to_string();
    let window_js = include_str!("window.js").to_string();
    let console_js = include_str!("console.js").to_string();
    let css = include_str!("style.css").to_string();
    let logo = include_str!("logo.svg").to_string();
    let logo_js = include_str!("logo.js").to_string();
    let menu_js = include_str!("menu.js").to_string();
    let grid_js = include_str!("grid.js").to_string();
    let rotary_js = include_str!("rotary.js").to_string();
    let control_js = include_str!("control.js").to_string();

    let window = Arc::new(window);    

    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )))]
    let _webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        
        let webview: Arc<std::sync::Mutex<Option<wry::WebView>>> = Arc::new(std::sync::Mutex::new(None));
        let webview_ipc = Arc::clone(&webview);
        
        let queue_clone = Arc::clone(&message_queue);
        let focused_grid_clone = Arc::clone(&focused_grid_element);
        let focused_menu_clone = Arc::clone(&focused_menu_option);
        
        let builder = WebViewBuilder::new()
            .with_url("app://local/index.html")
            .with_background_color((26, 26, 26, 255))
            .with_visible(false)
            .with_custom_protocol("app".into(), move |_webview_id, request| {
                use std::borrow::Cow;
                
                let path = request.uri().path();
                
                // Don't log the high-frequency message polling requests
                if path != "/messages" {
                    log::debug!("Custom protocol request: {}", request.uri());
                }
                
                if path == "/messages" {
                    // Poll endpoint for JavaScript to fetch pending messages
                    let mut queue = queue_clone.lock().unwrap();
                    let messages: Vec<String> = queue.drain(..).collect();
                    let json = serde_json::to_string(&messages).unwrap_or_else(|_| "[]".to_string());
                    
                    log::trace!("Sending {} messages to UI", messages.len());
                    
                    wry::http::Response::builder()
                        .header("Content-Type", "application/json")
                        .body(Cow::from(json.into_bytes()))
                        .unwrap()
                } else if path == "/style.css" {
                    wry::http::Response::builder()
                        .header("Content-Type", "text/css")
                        .body(Cow::from(css.clone().into_bytes()))
                        .unwrap()
                } else if path == "/index.html" {
                    wry::http::Response::builder()
                        .header("Content-Type", "text/html")
                        .body(Cow::from(window_html.clone().into_bytes()))
                        .unwrap()
                } else if path == "/window.js" {
                    wry::http::Response::builder()
                        .header("Content-Type", "application/javascript")
                        .body(Cow::from(window_js.clone().into_bytes()))
                        .unwrap()
                } else if path == "/console.js" {
                    wry::http::Response::builder()
                        .header("Content-Type", "application/javascript")
                        .body(Cow::from(console_js.clone().into_bytes()))
                        .unwrap()
                } else if path == "/logo.svg" {
                    wry::http::Response::builder()
                        .header("Content-Type", "image/svg+xml")
                        .body(Cow::from(logo.clone().into_bytes()))
                        .unwrap()
                } else if path == "/logo.js" {
                    wry::http::Response::builder()
                        .header("Content-Type", "application/javascript")
                        .body(Cow::from(logo_js.clone().into_bytes()))
                        .unwrap()
                } else if path == "/menu.js" {
                    wry::http::Response::builder()
                        .header("Content-Type", "application/javascript")
                        .body(Cow::from(menu_js.clone().into_bytes()))
                        .unwrap()
                } else if path == "/grid.js" {
                    wry::http::Response::builder()
                        .header("Content-Type", "application/javascript")
                        .body(Cow::from(grid_js.clone().into_bytes()))
                        .unwrap()
                } else if path == "/rotary.js" {
                    wry::http::Response::builder()
                        .header("Content-Type", "application/javascript")
                        .body(Cow::from(rotary_js.clone().into_bytes()))
                        .unwrap()
                } else if path == "/control.js" {
                    wry::http::Response::builder()
                        .header("Content-Type", "application/javascript")
                        .body(Cow::from(control_js.clone().into_bytes()))
                        .unwrap()
                } else if path == "/oxanium.ttf" {
                    // Include font file
                    let font_data = include_bytes!("oxanium.ttf");
                    wry::http::Response::builder()
                        .header("Content-Type", "font/ttf")
                        .body(Cow::from(font_data.as_slice()))
                        .unwrap()                    
                } else {
                    wry::http::Response::builder()
                        .status(404)
                        .body(Cow::from(Vec::new()))
                        .unwrap()
                }
            })
            .with_ipc_handler(move |req| {
                log::trace!("IPC message received: {}", req.body());
                
                // Parse the message from JavaScript
                if let Ok(message) = serde_json::from_str::<serde_json::Value>(req.body()) {
                    if let Some(msg_type) = message.get("type").and_then(|v| v.as_str()) {
                        match msg_type {
                            "console_log" => {
                                if let Some(msg) = message.get("data").and_then(|d| d.get("message")).and_then(|m| m.as_str()) {
                                    log::debug!("[JS] {}", msg);
                                }
                            }
                            "console_warn" => {
                                if let Some(msg) = message.get("data").and_then(|d| d.get("message")).and_then(|m| m.as_str()) {
                                    log::warn!("[JS] {}", msg);
                                }
                            }
                            "console_error" => {
                                if let Some(msg) = message.get("data").and_then(|d| d.get("message")).and_then(|m| m.as_str()) {
                                    log::error!("[JS] {}", msg);
                                }
                            }
                            "page_loaded" => {
                                log::info!("UI page loaded, making WebView visible");
                                if let Ok(wv) = webview_ipc.lock() {
                                    if let Some(wv) = wv.as_ref() {
                                        let _ = wv.set_visible(true);
                                    }
                                }
                            }
                            "menu_selected" => {
                                if let Some(data) = message.get("data") {
                                    log::debug!("Menu option selected: {:?}", data);
                                    // Controller will handle this via select() return value
                                }
                            }
                            "element_selected" => {
                                if let Some(data) = message.get("data") {
                                    log::debug!("Grid element selected: {:?}", data);
                                    // Controller will handle this via select() return value
                                }
                            }
                            "menu_closed" => {
                                log::debug!("Menu closed");
                            }
                            "focus_changed" => {
                                if let Some(data) = message.get("data") {
                                    if let Some(element_type) = data.get("type").and_then(|t| t.as_str()) {
                                        match element_type {
                                            "grid_node" => {
                                                if let Some(id) = data.get("id").and_then(|i| i.as_str()) {
                                                    let element = GridElement::Node(id.to_string());
                                                    log::trace!("Grid focus changed: {:?}", element);
                                                    *focused_grid_clone.lock().unwrap() = Some(element);
                                                }
                                            }
                                            "grid_link" => {
                                                if let (Some(from_id), Some(to_id), Some(link_type_str)) = (
                                                    data.get("fromId").and_then(|i| i.as_str()),
                                                    data.get("toId").and_then(|i| i.as_str()),
                                                    data.get("linkType").and_then(|t| t.as_str()),
                                                ) {
                                                    let link_type = match link_type_str {
                                                        "portIn" => LinkType::PortIn,
                                                        "portOut" => LinkType::PortOut,
                                                        "virtual" => LinkType::Virtual,
                                                        _ => LinkType::Normal,
                                                    };
                                                    let element = GridElement::Link(from_id.to_string(), to_id.to_string(), link_type);
                                                    log::trace!("Grid focus changed: {:?}", element);
                                                    *focused_grid_clone.lock().unwrap() = Some(element);
                                                }
                                            }
                                            "grid_none" => {
                                                log::trace!("Grid focus cleared");
                                                *focused_grid_clone.lock().unwrap() = None;
                                            }
                                            "menu" => {
                                                if let (Some(menu_id), Some(option_id)) = (
                                                    data.get("menuId").and_then(|i| i.as_str()),
                                                    data.get("optionId").and_then(|i| i.as_str()),
                                                ) {
                                                    let element = MenuOptionElement {
                                                        menu_id: menu_id.to_string(),
                                                        option_id: option_id.to_string(),
                                                    };
                                                    log::trace!("Menu focus changed: {:?}", element);
                                                    *focused_menu_clone.lock().unwrap() = Some(element);
                                                }
                                            }
                                            "menu_none" => {
                                                log::trace!("Menu focus cleared");
                                                *focused_menu_clone.lock().unwrap() = None;
                                            }
                                            _ => {
                                                log::warn!("Unknown element type in focus_changed: {}", element_type);
                                            }
                                        }
                                    }
                                }
                            }
                            "error" => {
                                if let Some(error) = message.get("data") {
                                    log::error!("JavaScript error: {:?}", error);
                                }
                            }
                            _ => {
                                log::warn!("Unknown IPC message type: {}", msg_type);
                            }
                        }
                    }
                } else {
                    // Legacy handling for simple string messages
                    log::debug!("Legacy IPC message: {}", req.body());
                    if let Ok(wv) = webview_ipc.lock() {
                        if let Some(wv) = wv.as_ref() {
                            let _ = wv.set_visible(true);
                        }
                    }
                }
            });
        
        let vbox = window.default_vbox().unwrap();
        let wv = builder.build_gtk(vbox)?;
        *webview.lock().unwrap() = Some(wv);
        webview
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::Quit) => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                window.set_visible(false);
                running.store(false, Ordering::SeqCst);
            }
            _ => {}
        }
    });
    
    #[allow(unreachable_code)]
    {
        //drop(_webview);
        Ok(())
    }
}
