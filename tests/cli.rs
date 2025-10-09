use std::process::Command;

use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};

mod zenoht;

fn cli() -> Command {
    Command::new(get_cargo_bin("zenoh-cli"))
}

#[test]
fn test_getting_a_non_existent_value() {
    assert_cmd_snapshot!(cli().args(["get", "foo"]));
}

#[test]
fn test_getting_a_value() {
    let _session = zenoht::builder()
        .with_queryable_key_value("foo", "bar")
        .start();

    assert_cmd_snapshot!(cli().args(["get", "foo"]));
}
