use insta_cmd::assert_cmd_snapshot;

mod zenoht;

#[test]
fn test_getting_a_non_existent_value() {
    let session = zenoht::builder()
        .add_storage(
            "test_getting_a_non_existent_value",
            zenoht::Storage::empty(),
        )
        .start();

    assert_cmd_snapshot!(
        session
            .cli()
            .args(["get", "test_getting_a_non_existent_value/foo"])
    );
}

#[test]
fn test_getting_a_value() {
    let session = zenoht::builder()
        .add_storage(
            "test_getting_a_value",
            zenoht::Storage::with_entries(&[("foo", "bar")]),
        )
        .start();

    assert_cmd_snapshot!(session.cli().args(["get", "test_getting_a_value/foo"]));
}
