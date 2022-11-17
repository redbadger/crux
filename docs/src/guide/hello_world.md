# Hello world

As the first step, we will build a simple application, starting with a classic Hello World, adding some state, and finally a remote API call. We will focus on the core, and return to the shell a little later.

If you want to follow along, you should start by following the [Shared core and types](../getting_started/core.md), guide to set up the project.

## Creating an app

To start with, we need a `struct` to be the root of our app.

```rust
#[derive(Default)]
pub struct Hello;
```

To turn it into an app, we need to implement the `App` trait from the `rmm` crate.

```rust

impl App for Hello {
    type Message = ();
    type Model = ();
    type ViewModel = String;

    fn update(&self, _message: Message, _model: &mut Model) -> Vec<Command<Msg>> {
        vec![Command::Render]
    }

    fn view(&self, model: &Model) -> ViewModel {
        "Hello World".to_string()
    }
}
```

The `view` function returns the representation of what we want the Shell to show on screen. To start with, when the Shell calls us with any message, we simply tell it to display the UI.

While technically that's the hello world and successfully implements the trait, it doesn't yet _do_ anything. We need to add the key components of an app: `Message` and `Model`.

## Counting up and down

To make things more interesting, we'll add some behaviour, specifically, we'll teach the app to count up and down. First, we'll need a model, which represents the state. We could just use a number, but we'll use a struct instead, so that we can easily add more state later.

```rust
#[derive(Default)]
struct Model {
    count: isize,
}
```

We need `Default` implemented to define the initial state. For now we derive it, as our state is quite simple. We also update the app to use the model:

```rust
impl App for Hello {
    type Message = ();
    type Model = Model;
    type ViewModel = String;

// ...

    fn view(&self, model: &Model) -> ViewModel {
        format!("Count is: {}", model.count)
    }
}
```

Great. All that's left is adding the behaviour. That's where `Message` comes in:

```rust
#[derive(Serialize, Deserialize)]
enum Message {
    Increment,
    Decrement,
    Reset,
}
```

The message covers all the possible events we can respond to. "Will that not get massive??" I hear you ask. Don't worry about that, there's [a solution to make this scale](./composing.md). Let's carry on. We need to actually handle those messages.

```rust
impl App for Hello {
    type Message = Message;
    type Model = Model;
    type ViewModel = String;

    fn update(&self, message: Message, model: &mut Model) -> Vec<Command<Msg>> {
        match message {
            Message::Increment => model.count += 1,
            Message::Decrement => model.count -= 1,
            Message::Reset => model.count = 0,
        };

        vec![Command::Render]
    }
// ...
```

Pretty straightforward, we just do what we're told, update the state, and tell the Shell to render. This would be a good time to write some tests to check everything works as expected.

```rust
#[cfg(test)]
mod test {
    use super::{Hello, Message};

    #[test]
    fn shows_initial_count() {
        todo!()
    }

    #[test]
    fn increments_count() {
        todo!()
    }

    #[test]
    fn decrements_count() {
        todo!()
    }

    #[test]
    fn resets_count() {
        todo!()
    }

    #[test]
    fn counts_up_and_down() {
        todo!()
    }
}
```

Hopefully those all pass. We are now sure that when we build an actual UI for this, it will _work_, and we'll be able to focus on making it looking delightful.

## Remote API

Before we dive into the details of the architecture, let's add one more feature - a remote API call - to get a little bit of a feel for how side-effects work.

**TO DO**
