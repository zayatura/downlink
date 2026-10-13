#![cfg(feature = "client")]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use oryx_layout::client::{FetchError, client_builder, fetch_layout, request_body, user_agent};
use oryx_protocol::LayoutIds;

fn ids() -> LayoutIds {
    LayoutIds::parse("abc12/rev99").unwrap()
}

/// Serves exactly one HTTP response and returns the raw request it received.
fn serve_once(status_line: &'static str, body: &'static str) -> (String, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/graphql", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut buf = [0u8; 4096];
        while !request_complete(&request) {
            let n = stream.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buf[..n]);
        }
        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
        String::from_utf8_lossy(&request).into_owned()
    });
    (url, handle)
}

fn request_complete(raw: &[u8]) -> bool {
    let text = String::from_utf8_lossy(raw);
    let Some(header_end) = text.find("\r\n\r\n") else {
        return false;
    };
    let length = text[..header_end]
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())?
        })
        .unwrap_or(0);
    raw.len() >= header_end + 4 + length
}

#[test]
fn request_body_names_the_layout_revision_and_geometry() {
    let body = request_body(&ids(), "voyager");
    assert_eq!(body["variables"]["hashId"], "abc12");
    assert_eq!(body["variables"]["revisionId"], "rev99");
    assert_eq!(body["variables"]["geometry"], "voyager");
    let query = body["query"].as_str().unwrap();
    assert!(
        query.contains("layout(hashId: $hashId, geometry: $geometry, revisionId: $revisionId)")
    );
    assert!(query.contains("layers { position title keys }"));
}

#[test]
fn user_agent_identifies_the_app_only() {
    assert_eq!(
        user_agent("1.2.3"),
        "Downlink/1.2.3 (+https://github.com/zayatura/downlink)"
    );
}

#[tokio::test]
async fn fetch_posts_graphql_and_returns_the_raw_body() {
    let (url, server) = serve_once("HTTP/1.1 200 OK", r#"{"data":{"layout":null}}"#);
    let client = client_builder("9.9.9").no_proxy().build().unwrap();
    let body = fetch_layout(&client, &url, &ids(), "voyager")
        .await
        .expect("fetch succeeds");
    assert_eq!(body, r#"{"data":{"layout":null}}"#);
    let request = server.join().unwrap();
    assert!(request.starts_with("POST /graphql"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("user-agent: downlink/9.9.9")
    );
    assert!(request.contains(r#""revisionId":"rev99""#));
}

#[tokio::test]
async fn http_errors_are_reported_with_their_status() {
    let (url, server) = serve_once("HTTP/1.1 503 Service Unavailable", "{}");
    let client = client_builder("9.9.9").no_proxy().build().unwrap();
    let error = fetch_layout(&client, &url, &ids(), "voyager")
        .await
        .unwrap_err();
    assert!(matches!(error, FetchError::Status(503)), "{error:?}");
    server.join().unwrap();
}
