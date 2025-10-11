use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use insta_cmd::assert_cmd_snapshot;
use zenoh::bytes::ZBytes;

mod zenoht;

#[test]
fn test_getting_a_non_existent_value() {
    let session = zenoht::builder()
        .add_storage("test", zenoht::Storage::empty())
        .start();

    assert_cmd_snapshot!(session.cli().args(["get", "test/foo"]));
}

#[test]
fn test_getting_a_value() {
    let session = zenoht::builder()
        .add_storage("test", zenoht::Storage::with_entries(&[("foo", "bar")]))
        .start();

    assert_cmd_snapshot!(session.cli().args(["get", "test/foo"]));
}

#[test]
fn test_putting_a_value() {
    let storage = zenoht::Storage::empty();
    let session = zenoht::builder()
        .add_storage("test", storage.clone())
        .start();

    assert_cmd_snapshot!(session.cli().args(["put", "test/foo", "bar"]));

    let value = session.block_on(async { storage.get("foo").await });
    assert_eq!(value, Some(ZBytes::from("bar")));
}

#[test]
fn test_deleting_a_value() {
    let storage = zenoht::Storage::with_entries(&[("foo", "bar")]);
    let session = zenoht::builder()
        .add_storage("test", storage.clone())
        .start();

    let value = session.block_on(async { storage.get("foo").await });
    assert_eq!(value, Some(ZBytes::from("bar")));

    assert_cmd_snapshot!(session.cli().args(["del", "test/foo"]));

    let value = session.block_on(async { storage.get("foo").await });
    assert_eq!(value, None);
}

#[test]
fn test_subscribing_to_a_keyexpr() {
    let session = zenoht::builder().start();

    let mut child = session
        .cli()
        .args(["sub", "test/**"])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    session.wait_for_peer();
    session.put("test/foo", "bar");
    session.put("test/baz", "qux");

    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut lines = BufReader::new(stdout).lines();

        let line = lines.next().unwrap().unwrap();
        assert!(line.contains("test/foo"));
        assert!(line.contains("bar"));

        let line = lines.next().unwrap().unwrap();
        assert!(line.contains("test/baz"));
        assert!(line.contains("qux"));

        tx.send(()).unwrap();
    });

    rx.recv_timeout(Duration::from_secs(10))
        .expect("failed to receive sent messages");

    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn test_getting_zid() {
    let session = zenoht::builder()
        .with_cli_config("id", r#""102030405060708090a0b0c0d0e0f10""#)
        .start();

    assert_cmd_snapshot!(session.cli().arg("zid"));
}

#[test]
fn test_getting_peers() {
    let session = zenoht::builder()
        .with_router_config("id", r#""202030405060708090a0b0c0d0e0f10""#)
        .start();

    assert_cmd_snapshot!(session.cli().arg("peers"));
}

#[test]
fn test_getting_routers() {
    let session = zenoht::builder()
        .with_router_config("id", r#""202030405060708090a0b0c0d0e0f10""#)
        .start();

    assert_cmd_snapshot!(session.cli().arg("routers"));
}
