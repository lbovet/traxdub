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
        .with_inner_size(LogicalSize::new(800.0, 600.0))
        .build(&event_loop)?;

    let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Oxanium:wght@200&display=swap');
        
        body {
            margin: 0;
            padding: 0;
            width: 100vw;
            height: 100vh;
            background: #1a1a1a;
            display: flex;
            align-items: center;
            justify-content: center;
            font-family: 'Oxanium', sans-serif;
        }
        
        .welcome {
            font-size: 2rem;
            font-weight: 200;
            color: #d4c5a0;
            text-align: center;
        }
    </style>
</head>
<body>
    <div class="welcome">Welcome</div>
</body>
</html>"#;

    let builder = WebViewBuilder::new()
        .with_url(&format!("data:text/html,{}", urlencoding::encode(html)));

    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    ))]
    let _webview = builder.build(&window)?;

    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )))]
    let _webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().unwrap();
        builder.build_gtk(vbox)?
    };

    window.set_visible(true);

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
                running.store(false, Ordering::SeqCst);
            }
            _ => {}
        }
    });
    
    #[allow(unreachable_code)]
    {
        drop(_webview);
        Ok(())
    }
}
