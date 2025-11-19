use super::super::Command;

enum Effect {}

#[derive(Debug, PartialEq)]
enum Event {}

#[derive(Default)]
struct Model {
    test: String
}

// Commands can be constructed without async and dispatch basic
// effects, which are executed lazily when the command is asked for
// emitted events or effects

#[test]
fn mutate_model() {
    let mut cmd = Command::<Effect, Event, Model>::new_with_model(|ctx| async move {
        ctx.model(|model| model.test = "Hello World".to_owned()).await;

        let value = ctx.model(|model| model.test.clone()).await;

        assert_eq!(value, "Hello World");
    });

    let mut model = Model::default();

    cmd.run_until_settled(Some(&mut model));

    assert_eq!(model.test, "Hello World");
}
