use std::{io::Read, process::Stdio, time::Duration};

use insta::assert_snapshot;
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

    std::thread::sleep(Duration::from_millis(500));

    let mut stderr = child.stderr.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    child.kill().unwrap();
    child.wait().unwrap();

    let mut stdout_buf = String::new();
    stdout.read_to_string(&mut stdout_buf).unwrap();

    let mut stderr_buf = String::new();
    stderr.read_to_string(&mut stderr_buf).unwrap();

    assert_snapshot!(format!(
        "----- stdout -----\n{}\n----- stderr -----\n{}",
        stdout_buf, stderr_buf,
    ));
}
