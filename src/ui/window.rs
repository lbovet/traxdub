use anyhow::Result;

/// Create and run the UI window  
pub fn run() -> Result<()> {
    use wry::{
        dpi::LogicalSize,
        WebViewBuilder,
    };
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    };

    let event_loop = EventLoop::new();
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
        @import url('https://fonts.googleapis.com/css2?family=Oxanium:wght@400;600;700&display=swap');
        
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
            font-size: 4rem;
            font-weight: 600;
            color: #d4c5a0;
            text-align: center;
        }
    </style>
</head>
<body>
    <div class="welcome">welcome</div>
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

        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event {
            *control_flow = ControlFlow::Exit;
        }
    });
    
    #[allow(unreachable_code)]
    {
        drop(_webview);
        Ok(())
    }
}
