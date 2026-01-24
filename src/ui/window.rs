use anyhow::Result;
use std::sync::{Arc, OnceLock, atomic::{AtomicBool, Ordering}};
use tao::event_loop::EventLoopProxy;

#[derive(Debug, Clone)]
pub enum UserEvent {
    Quit,
}

static PROXY: OnceLock<EventLoopProxy<UserEvent>> = OnceLock::new();

pub fn close() -> Result<()> {
    if let Some(proxy) = PROXY.get() {
        proxy.send_event(UserEvent::Quit)
            .map_err(|e| anyhow::anyhow!("Failed to send quit event: {:?}", e))?;
    }
    Ok(())
}

/// Create and run the UI window  
pub fn run(running: Arc<AtomicBool>) -> Result<()> {
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

    let html = include_str!("window.html").to_string();
    let css = include_str!("style.css").to_string();

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
        let webview_clone = Arc::clone(&webview);
        
        let builder = WebViewBuilder::new()
            .with_url("app://local/index.html")
            .with_background_color((26, 26, 26, 255))
            .with_visible(false)
            .with_custom_protocol("app".into(), move |_webview_id, request| {
                use std::borrow::Cow;
                
                log::debug!("Custom protocol request: {}", request.uri());
                
                if request.uri().path() == "/style.css" {
                    wry::http::Response::builder()
                        .header("Content-Type", "text/css")
                        .body(Cow::from(css.clone().into_bytes()))
                        .unwrap()
                } else if request.uri().path() == "/index.html" {
                    wry::http::Response::builder()
                        .header("Content-Type", "text/html")
                        .body(Cow::from(html.clone().into_bytes()))
                        .unwrap()
                } else if request.uri().path() == "/oxanium.ttf" {
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
            .with_ipc_handler(move |_req| {
                if let Ok(wv) = webview_clone.lock() {
                    if let Some(wv) = wv.as_ref() {
                        let _ = wv.set_visible(true);
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
