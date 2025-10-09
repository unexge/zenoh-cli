use std::process::Command;

use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};

mod zenoht;

fn cli() -> Command {
    Command::new(get_cargo_bin("zenoh-cli"))
}

#[test]
fn test_getting_a_non_existent_value() {
    let _session = zenoht::builder()
        .add_storage(
            "test_getting_a_non_existent_value",
            zenoht::Storage::empty(),
        )
        .start();

    assert_cmd_snapshot!(cli().args(["get", "test_getting_a_non_existent_value/foo"]));
}

#[test]
fn test_getting_a_value() {
    let _session = zenoht::builder()
        .add_storage(
            "test_getting_a_value",
            zenoht::Storage::with_entries(&[("foo", "bar")]),
        )
        .start();

    assert_cmd_snapshot!(cli().args(["get", "test_getting_a_value/foo"]));
}
