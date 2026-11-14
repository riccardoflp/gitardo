use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use crate::test_server::{AUTHORITY, OTHER_AUTHORITY, accept};
use crate::{Error, TlsStream, Trust};

/// A server on 127.0.0.1 that accepts one connection, makes it secure, and sends back in capitals the line it reads.
/// It gives what its side of the handshake ended with.
fn shouting_server() -> (u16, thread::JoinHandle<Result<(), Error>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (tcp, _) = listener.accept().unwrap();
        let mut stream = accept(tcp)?;
        let mut line = Vec::new();
        let mut byte = [0_u8];
        while stream.read(&mut byte).unwrap() == 1 && byte[0] != b'\n' {
            line.push(byte[0]);
        }
        stream.write_all(&line.to_ascii_uppercase()).unwrap();
        stream.flush().unwrap();
        Ok(())
    });
    (port, server)
}

#[test]
fn a_server_signed_by_a_trusted_authority_is_reached() {
    let (port, server) = shouting_server();
    let trust = Trust::from_pem(AUTHORITY).unwrap();
    let tcp = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let mut stream = TlsStream::connect(&trust, "localhost", tcp).unwrap();
    stream.write_all(b"hello\n").unwrap();
    stream.flush().unwrap();
    let mut answer = Vec::new();
    let _ = stream.read_to_end(&mut answer).unwrap();
    assert_eq!(answer, b"HELLO");
    server.join().unwrap().unwrap();
}

#[test]
fn an_address_named_in_the_certificate_is_reached_too() {
    let (port, server) = shouting_server();
    let trust = Trust::from_pem(AUTHORITY).unwrap();
    let tcp = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let mut stream = TlsStream::connect(&trust, "127.0.0.1", tcp).unwrap();
    stream.write_all(b"ok\n").unwrap();
    let mut answer = Vec::new();
    let _ = stream.read_to_end(&mut answer).unwrap();
    assert_eq!(answer, b"OK");
    server.join().unwrap().unwrap();
}

#[test]
fn a_server_signed_by_another_authority_is_not_trusted() {
    let (port, server) = shouting_server();
    let trust = Trust::from_pem(OTHER_AUTHORITY).unwrap();
    let tcp = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let error = TlsStream::connect(&trust, "localhost", tcp).unwrap_err();
    assert!(
        matches!(&error, Error::Untrusted { host, .. } if host == "localhost"),
        "{error:?}"
    );
    assert!(
        error
            .to_string()
            .starts_with("The certificate of localhost is not trusted by this system")
    );
    assert!(server.join().unwrap().is_err());
}

#[test]
fn a_certificate_for_another_name_is_not_trusted() {
    let (port, server) = shouting_server();
    let trust = Trust::from_pem(AUTHORITY).unwrap();
    let tcp = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let error = TlsStream::connect(&trust, "elsewhere.test", tcp).unwrap_err();
    assert!(
        matches!(&error, Error::Untrusted { host, .. } if host == "elsewhere.test"),
        "{error:?}"
    );
    assert!(server.join().unwrap().is_err());
}

#[test]
fn a_name_that_is_no_host_is_refused_before_anything_is_sent() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let tcp = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let trust = Trust::from_pem(AUTHORITY).unwrap();
    let error = TlsStream::connect(&trust, "not a host", tcp).unwrap_err();
    assert!(matches!(error, Error::BadHostName { .. }), "{error:?}");
}

#[test]
fn a_file_with_no_certificate_is_refused() {
    let error = Trust::from_pem(b"nothing here\n").unwrap_err();
    assert!(matches!(error, Error::BadCertificates { .. }), "{error:?}");
}

#[test]
fn the_system_gives_root_certificates() {
    assert!(Trust::system().is_ok());
}
