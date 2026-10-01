use crux_core::App as _;
use shared::*;

#[test]
fn increments_count() {
    let app = Counter;
    let mut model = Model::default();

    app.update(Event::Increment, &mut model)
        .expect_only_render();

    let actual_view = app.view(&model).count;
    let expected_view = "Count is: 1";
    assert_eq!(actual_view, expected_view);
}
