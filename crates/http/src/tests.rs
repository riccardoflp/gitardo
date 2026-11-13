//! Requests sent to the servers of the tests on 127.0.0.1, in the clear and over TLS, directly and through a proxy.

use std::io::BufReader;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::test_server::{Answer, TestServer, answer};
use crate::{Cancel, Error, Events, Http, Proxies, ProxySettings, Request, Url};

fn plain_http() -> Http {
    Http::new(TestServer::trust(), Proxies::none())
}

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

fn local(server: &TestServer, scheme: &str, path: &str) -> Url {
    url(&format!("{scheme}://127.0.0.1:{}{path}", server.port()))
}

#[test]
fn a_get_reads_the_status_the_headers_and_the_body() {
    let server = TestServer::plain(vec![Answer::Bytes(answer("200 OK", &[("X-Kind", "test")], b"hello"))]);
    let response = plain_http()
        .send(&Request::get(local(&server, "http", "/path?q=1")), &Cancel::new())
        .unwrap();
    assert_eq!(response.head.status_line(), "200 OK");
    assert_eq!(response.head.header("x-kind"), Some("test"));
    assert_eq!(response.body, b"hello");
    let received = server.received();
    let request = received.first().unwrap();
    assert_eq!(request.request_line(), "GET /path?q=1 HTTP/1.1");
    assert_eq!(request.header("host"), Some(format!("127.0.0.1:{}", server.port())));
    assert_eq!(request.header("connection").as_deref(), Some("close"));
    assert_eq!(request.header("accept-encoding"), None);
    assert_eq!(request.header("content-length"), None);
}

#[test]
fn a_post_sends_its_body_with_its_length() {
    let server = TestServer::plain(vec![Answer::Bytes(answer("201 Created", &[], b""))]);
    let request = Request::post(local(&server, "http", "/graphql"), b"{\"a\":1}".to_vec())
        .header("Content-Type", "application/json");
    let response = plain_http().send(&request, &Cancel::new()).unwrap();
    assert_eq!(response.head.status, 201);
    let received = server.received();
    let request = received.first().unwrap();
    assert_eq!(request.request_line(), "POST /graphql HTTP/1.1");
    assert_eq!(request.header("content-length").as_deref(), Some("7"));
    assert_eq!(request.header("content-type").as_deref(), Some("application/json"));
    assert_eq!(request.body, b"{\"a\":1}");
}

#[test]
fn a_chunked_body_and_one_that_ends_with_the_connection() {
    let chunked = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n".to_vec();
    let until_close = b"HTTP/1.0 200 OK\r\n\r\nall of it".to_vec();
    let server = TestServer::plain(vec![Answer::Bytes(chunked), Answer::Bytes(until_close)]);
    let http = plain_http();
    let first = http
        .send(&Request::get(local(&server, "http", "/")), &Cancel::new())
        .unwrap();
    assert_eq!(first.body, b"Wikipedia");
    let second = http
        .send(&Request::get(local(&server, "http", "/")), &Cancel::new())
        .unwrap();
    assert_eq!(second.body, b"all of it");
}

#[test]
fn an_answer_that_says_more_is_coming_is_not_the_answer() {
    let mut bytes = b"HTTP/1.1 100 Continue\r\n\r\n".to_vec();
    bytes.extend(answer("200 OK", &[], b"done"));
    let server = TestServer::plain(vec![Answer::Bytes(bytes)]);
    let response = plain_http()
        .send(&Request::get(local(&server, "http", "/")), &Cancel::new())
        .unwrap();
    assert_eq!((response.head.status, response.body.as_slice()), (200, &b"done"[..]));
}

#[test]
fn a_secret_header_is_sent_but_never_shown() {
    let server = TestServer::plain(vec![Answer::Bytes(b"not http\r\n\r\n".to_vec())]);
    let request = Request::get(local(&server, "http", "/")).secret_header("Authorization", "Bearer sekrit-token");
    assert!(!format!("{request:?}").contains("sekrit"));
    let error = plain_http().send(&request, &Cancel::new()).unwrap_err();
    assert!(matches!(error, Error::BadResponse { .. }), "{error:?}");
    assert!(!error.to_string().contains("sekrit") && !format!("{error:?}").contains("sekrit"));
    let received = server.received();
    assert_eq!(
        received.first().unwrap().header("authorization").as_deref(),
        Some("Bearer sekrit-token")
    );
}

#[test]
fn https_is_spoken_with_a_server_the_roots_trust() {
    let server = TestServer::secure(vec![Answer::Bytes(answer("200 OK", &[], b"secure"))]);
    let response = plain_http()
        .send(&Request::get(local(&server, "https", "/")), &Cancel::new())
        .unwrap();
    assert_eq!(response.body, b"secure");
}

#[test]
fn a_server_the_roots_do_not_trust_is_refused_in_words() {
    let server = TestServer::secure(vec![Answer::Bytes(answer("200 OK", &[], b"secure"))]);
    let http = Http::new(TestServer::other_trust(), Proxies::none());
    let error = http
        .send(&Request::get(local(&server, "https", "/")), &Cancel::new())
        .unwrap_err();
    assert!(matches!(error, Error::Tls(tls::Error::Untrusted { .. })), "{error:?}");
    assert!(error.to_string().contains("is not trusted by this system"));
}

#[test]
fn an_address_given_for_a_host_is_reached_with_the_host_s_name() {
    let server = TestServer::secure(vec![Answer::Bytes(answer("200 OK", &[], b"{}"))]);
    let http = plain_http().with_address("api.github.com", 443, server.address());
    let response = http
        .send(&Request::get(url("https://api.github.com/graphql")), &Cancel::new())
        .unwrap();
    assert_eq!(response.body, b"{}");
    assert_eq!(
        server.received().first().unwrap().header("host").as_deref(),
        Some("api.github.com")
    );
}

#[test]
fn redirects_are_followed_to_https() {
    let server = TestServer::secure(vec![
        Answer::Bytes(answer("302 Found", &[("Location", "/moved")], b"")),
        Answer::Bytes(answer(
            "308 Permanent Redirect",
            &[("Location", "https://example.test/final")],
            b"",
        )),
        Answer::Bytes(answer("200 OK", &[], b"arrived")),
    ]);
    let http = plain_http().with_address("example.test", 443, server.address());
    let request = Request::get(local(&server, "https", "/start")).secret_header("Authorization", "Bearer token");
    let response = http.send(&request, &Cancel::new()).unwrap();
    assert_eq!(response.body, b"arrived");
    let received = server.received();
    let lines: Vec<String> = received
        .iter()
        .map(super::test_server::Received::request_line)
        .collect();
    assert_eq!(
        lines,
        ["GET /start HTTP/1.1", "GET /moved HTTP/1.1", "GET /final HTTP/1.1"]
    );
    let authorizations: Vec<Option<String>> = received.iter().map(|request| request.header("authorization")).collect();
    assert_eq!(
        authorizations,
        [
            Some(String::from("Bearer token")),
            Some(String::from("Bearer token")),
            None
        ],
        "the token goes to the host it was meant for, not to another"
    );
}

#[test]
fn a_post_sent_elsewhere_with_303_becomes_a_get_without_its_body() {
    let server = TestServer::secure(vec![
        Answer::Bytes(answer("303 See Other", &[("Location", "/result")], b"")),
        Answer::Bytes(answer("200 OK", &[], b"ok")),
    ]);
    let request =
        Request::post(local(&server, "https", "/send"), b"body".to_vec()).header("Content-Type", "text/plain");
    let _ = plain_http().send(&request, &Cancel::new()).unwrap();
    let received = server.received();
    let second = received.get(1).unwrap();
    assert_eq!(second.request_line(), "GET /result HTTP/1.1");
    assert_eq!(
        (second.body.as_slice(), second.header("content-type")),
        (&b""[..], None)
    );
}

#[test]
fn a_redirect_to_http_is_not_followed() {
    let server = TestServer::plain(vec![Answer::Bytes(answer(
        "301 Moved",
        &[("Location", "/elsewhere")],
        b"",
    ))]);
    let error = plain_http()
        .send(&Request::get(local(&server, "http", "/")), &Cancel::new())
        .unwrap_err();
    assert!(matches!(error, Error::InsecureRedirect { .. }), "{error:?}");
    assert_eq!(server.received().len(), 1);
}

#[test]
fn more_than_five_redirects_are_too_many() {
    let again = || Answer::Bytes(answer("302 Found", &[("Location", "/again")], b""));
    let server = TestServer::secure((0..6).map(|_| again()).collect());
    let error = plain_http()
        .send(&Request::get(local(&server, "https", "/")), &Cancel::new())
        .unwrap_err();
    assert!(matches!(error, Error::TooManyRedirects { .. }), "{error:?}");
    assert_eq!(server.received().len(), 6);
}

fn through(proxy: &TestServer, user: &str, no_proxy: Option<&str>) -> Http {
    let settings = ProxySettings {
        https_proxy: Some(format!("http://{user}127.0.0.1:{}", proxy.port())),
        git_http_proxy: None,
        no_proxy: no_proxy.map(str::to_owned),
    };
    Http::new(TestServer::trust(), Proxies::from_settings(&settings).unwrap())
}

#[test]
fn a_proxy_opens_a_tunnel_with_connect_and_basic_authentication() {
    let host_answer = Answer::Bytes(answer("200 OK", &[], b"through the proxy"));
    let proxy = TestServer::plain(vec![Answer::Tunnel(Box::new(host_answer))]);
    let http = through(&proxy, "ada:open%20sesame@", None);
    let response = http
        .send(&Request::get(url("https://api.github.com/graphql")), &Cancel::new())
        .unwrap();
    assert_eq!(response.body, b"through the proxy");
    let received = proxy.received();
    let connect = received.first().unwrap();
    assert_eq!(connect.request_line(), "CONNECT api.github.com:443 HTTP/1.1");
    assert_eq!(connect.header("host").as_deref(), Some("api.github.com:443"));
    assert_eq!(
        connect.header("proxy-authorization").as_deref(),
        Some("Basic YWRhOm9wZW4gc2VzYW1l")
    );
    let request = received.get(1).unwrap();
    assert_eq!(request.request_line(), "GET /graphql HTTP/1.1");
    assert_eq!(request.header("proxy-authorization"), None);
}

#[test]
fn a_proxy_that_refuses_says_so_without_its_password() {
    let refusal = answer("407 Proxy Authentication Required", &[], b"");
    let proxy = TestServer::plain(vec![Answer::Bytes(refusal)]);
    let http = through(&proxy, "ada:hidden-word@", None);
    let error = http
        .send(&Request::get(url("https://example.test/")), &Cancel::new())
        .unwrap_err();
    assert!(matches!(&error, Error::Proxy { status, .. } if status == "407 Proxy Authentication Required"));
    assert!(!error.to_string().contains("hidden-word") && !format!("{error:?}").contains("hidden-word"));
}

#[test]
fn a_host_no_proxy_names_is_reached_directly() {
    let server = TestServer::plain(vec![Answer::Bytes(answer("200 OK", &[], b"direct"))]);
    let proxy = TestServer::plain(vec![]);
    let http = through(&proxy, "", Some("127.0.0.1"));
    let response = http
        .send(&Request::get(local(&server, "http", "/")), &Cancel::new())
        .unwrap();
    assert_eq!(response.body, b"direct");
    assert_eq!(proxy.received(), []);
}

#[test]
fn a_request_cancelled_while_it_waits_ends_at_once() {
    let server = TestServer::plain(vec![Answer::Silence]);
    let cancel = Cancel::new();
    let canceller = cancel.clone();
    let started = Instant::now();
    let _thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        canceller.cancel();
    });
    let error = plain_http()
        .send(&Request::get(local(&server, "http", "/")), &cancel)
        .unwrap_err();
    assert!(matches!(error, Error::Cancelled), "{error:?}");
    assert!(started.elapsed() < Duration::from_secs(5));
    let error = plain_http()
        .send(&Request::get(local(&server, "http", "/")), &cancel)
        .unwrap_err();
    assert!(
        matches!(error, Error::Cancelled),
        "a request sent once it is cancelled does not start"
    );
}

#[test]
fn a_cancel_can_follow_the_token_of_a_job() {
    let server = TestServer::plain(vec![Answer::Silence]);
    let job = Cancel::new();
    let watched = job.clone();
    let cancel = Cancel::following(move || watched.is_cancelled());
    let _thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        job.cancel();
    });
    let error = plain_http()
        .send(&Request::get(local(&server, "http", "/")), &cancel)
        .unwrap_err();
    assert!(matches!(error, Error::Cancelled), "{error:?}");
}

#[test]
fn a_server_that_says_nothing_times_out() {
    let server = TestServer::plain(vec![Answer::Silence]);
    let request = Request::get(local(&server, "http", "/")).idle_timeout(Duration::from_millis(200));
    let error = plain_http().send(&request, &Cancel::new()).unwrap_err();
    assert!(matches!(error, Error::TimedOut { .. }), "{error:?}");
}

#[test]
fn a_host_nobody_listens_on_cannot_be_connected_to() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let error = plain_http()
        .send(&Request::get(url(&format!("http://127.0.0.1:{port}/"))), &Cancel::new())
        .unwrap_err();
    assert!(
        matches!(error, Error::Connect { .. } | Error::TimedOut { .. }),
        "{error:?}"
    );
}

#[test]
fn server_sent_events_are_read_as_they_arrive() {
    let (go_on, wait) = mpsc::channel();
    let head = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n";
    let mut first = head.to_vec();
    first.extend_from_slice(b"f\r\ndata: {\"n\":1}\n\n\r\n");
    let rest = b"f\r\ndata: {\"n\":2}\n\n\r\n0\r\n\r\n".to_vec();
    let server = TestServer::secure(vec![Answer::Parts(first, wait, rest)]);
    let opened = plain_http()
        .open(&Request::get(local(&server, "https", "/stream")), &Cancel::new())
        .unwrap();
    assert_eq!(opened.head.header("content-type"), Some("text/event-stream"));
    let mut events = Events::new(BufReader::new(opened.body));
    assert_eq!(
        events.next_event().unwrap().unwrap().data,
        "{\"n\":1}",
        "before the rest is sent"
    );
    go_on.send(()).unwrap();
    assert_eq!(events.next_event().unwrap().unwrap().data, "{\"n\":2}");
    assert_eq!(events.next_event().unwrap(), None);
}
