#[tokio::main]
async fn main() {}

#[cfg(test)]
mod tests {
    fn build_frontend() {
        let frontend_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/frontend");
        let t = Instant::now();
        eprintln!("[bench] building frontend: {frontend_dir} (npm run build)...");

        let status = Command::new("bash")
            .args(["-c", "npm run build"])
            .current_dir(frontend_dir)
            .status()
            .expect("Failed to run npm build");

        assert!(status.success(), "Frontend build failed");
        eprintln!("[bench] frontend build: {:.1}s", t.elapsed().as_secs_f64());
    }

    async fn start_http_server(html: String, webrtc_port: u16, fingerprint: String) -> u16 {
        let info = serde_json::json!({
            "port": webrtc_port,
            "fingerprint": fingerprint,
        });

        let app = Router::new()
            .route(
                "/",
                get({
                    let html = html.clone();
                    move || {
                        let html = html.clone();
                        async move { Html(html) }
                    }
                }),
            )
            .route(
                "/webrtc-info",
                get(move || {
                    let info = info.clone();
                    async move { Json(info) }
                }),
            );

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind HTTP server");
        let http_port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        http_port
    }

    fn install_test_result_listener(tab: &Tab) {
        tab.call_method(headless_chrome::protocol::cdp::Page::AddScriptToEvaluateOnNewDocument {
            source: r#"
                    window.__testResult = { status: 'pending' };
                    window.__consoleLogs = [];
                    window.__consoleErrors = [];
                    window.__logIndex = 0;
                    window.__errIndex = 0;
                    const origLog = console.log;
                    const origErr = console.error;
                    const origWarn = console.warn;
                    console.log = function(...args) { window.__consoleLogs.push(args.map(String).join(' ')); origLog.apply(console, args); };
                    console.error = function(...args) { window.__consoleErrors.push(args.map(String).join(' ')); origErr.apply(console, args); };
                    console.warn = function(...args) { window.__consoleErrors.push('WARN: ' + args.map(String).join(' ')); origWarn.apply(console, args); };
                    window.addEventListener('unhandledrejection', (event) => {
                        window.__consoleErrors.push('UNHANDLED REJECTION: ' + String(event.reason));
                    });
                    window.addEventListener('error', (event) => {
                        window.__consoleErrors.push('UNCAUGHT: ' + event.message + ' at ' + event.filename + ':' + event.lineno);
                    });
                    window.addEventListener('message', (event) => {
                        if (event.data && event.data.type === 'test-result') {
                            window.__testResult.status = event.data.status;
                        }
                    });
                "#
            .to_string(),
            world_name: None,
            include_command_line_api: None,
            run_immediately: None,
        })
        .expect("Failed to install test result listener");
    }

    fn dump_new_logs(tab: &Tab, elapsed: f64) {
        if let Ok(result) = tab.evaluate(
            r#"(() => {
                const i = window.__logIndex || 0;
                const logs = (window.__consoleLogs || []).slice(i);
                window.__logIndex = (window.__consoleLogs || []).length;
                return logs.join('\n');
            })()"#,
            false,
        ) {
            if let Some(s) = result.value.as_ref().and_then(|v| v.as_str()) {
                if !s.is_empty() {
                    eprintln!("[{elapsed:.1}s console] {s}");
                }
            }
        }
        if let Ok(result) = tab.evaluate(
            r#"(() => {
                const i = window.__errIndex || 0;
                const errs = (window.__consoleErrors || []).slice(i);
                window.__errIndex = (window.__consoleErrors || []).length;
                return errs.join('\n');
            })()"#,
            false,
        ) {
            if let Some(s) = result.value.as_ref().and_then(|v| v.as_str()) {
                if !s.is_empty() {
                    eprintln!("[{elapsed:.1}s ERRORS] {s}");
                }
            }
        }
    }

    fn await_test_result(tab: &Tab) {
        let start = Instant::now();
        let poll_interval = Duration::from_millis(500);
        let mut polls = 0;
        loop {
            let elapsed = start.elapsed().as_secs_f64();
            let result = tab
                .evaluate(r#"window.__testResult.status"#, false)
                .expect("Failed to read test status");

            let status = result.value.as_ref().and_then(|v| v.as_str());
            match status {
                Some("success") => {
                    dump_new_logs(tab, elapsed);
                    eprintln!("[{elapsed:.1}s] TEST PASSED");
                    return;
                }
                Some("failed") => {
                    dump_new_logs(tab, elapsed);
                    panic!("[{elapsed:.1}s] Frontend stress tests failed");
                }
                _ => {
                    polls += 1;
                    if polls % 4 == 1 {
                        dump_new_logs(tab, elapsed);
                    }
                    std::thread::sleep(poll_interval);
                }
            }
        }
    }

    async fn run_browser_test(url: &str, timeout_secs: u64) {
        let total = Instant::now();
        let url = url.to_string();
        eprintln!("[bench] launching browser for {url} (timeout={timeout_secs}s)...");
        let result = timeout(
            Duration::from_secs(timeout_secs),
            spawn_blocking(move || {
                let t = Instant::now();
                let browser = Browser::new(
                    LaunchOptions::default_builder()
                        .idle_browser_timeout(Duration::from_secs(360))
                        .build()
                        .unwrap(),
                )
                .expect("Failed to launch browser");
                let tab = browser.new_tab().expect("Failed to create tab");
                eprintln!("[bench] browser launch: {:.1}s", t.elapsed().as_secs_f64());

                install_test_result_listener(&tab);

                let t = Instant::now();
                eprintln!("[bench] navigating to {url}...");
                tab.navigate_to(&url).expect("Failed to navigate");
                if let Err(e) = tab.wait_until_navigated() {
                    eprintln!("[bench] wait_until_navigated failed (continuing): {e}");
                }
                eprintln!("[bench] navigation: {:.1}s", t.elapsed().as_secs_f64());

                let t = Instant::now();
                eprintln!("[bench] polling for test result...");
                await_test_result(&tab);
                eprintln!("[bench] test execution: {:.1}s", t.elapsed().as_secs_f64());
            }),
        )
        .await;

        eprintln!("[bench] run_browser_test total: {:.1}s", total.elapsed().as_secs_f64());
        match result {
            Ok(join_result) => join_result.expect("Browser task panicked"),
            Err(_) => panic!("Test timed out after {}s", timeout_secs),
        }
    }

    #[tokio::test]
    #[serial]
    async fn stress_test_webrtc_direct() {
        let total = Instant::now();

        let (webrtc_port, fingerprint) = start_echo_server().await;
        eprintln!("[bench] echo server on port {webrtc_port}, fingerprint: {fingerprint}");

        build_frontend();

        let html_path = concat!(env!("CARGO_MANIFEST_DIR"), "/frontend/dist/index.html");
        let html = std::fs::read_to_string(html_path).expect("Failed to read built frontend HTML");
        eprintln!("[bench] HTML size: {} bytes", html.len());

        let http_port = start_http_server(html, webrtc_port, fingerprint).await;
        let url = format!("http://127.0.0.1:{http_port}");
        eprintln!("[bench] HTTP server at {url}");

        run_browser_test(&url, 60).await;

        eprintln!(
            "\n[bench] === stress_test_webrtc_direct TOTAL: {:.1}s ===\n",
            total.elapsed().as_secs_f64()
        );
    }

    use std::process::Command;
    use std::time::{Duration, Instant};

    use axum::response::Html;
    use axum::routing::get;
    use axum::{Json, Router};
    use headless_chrome::{Browser, LaunchOptions, Tab};
    use serial_test::serial;
    use tokio::net::TcpListener;
    use tokio::task::spawn_blocking;
    use tokio::time::timeout;
    use webrtc_direct_integration_tests::echo_server::start_echo_server;
}
