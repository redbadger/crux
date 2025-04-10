mod payment;

use crux_core::{
    macros::effect,
    render::{render, RenderOperation},
    Command,
};
use serde::{Deserialize, Serialize};

use crate::{capabilities::delay::Delay, DelayOperation};
pub use payment::{Payment, PaymentStatus, Receipt, ReceiptStatus};

#[derive(Serialize, Deserialize, Debug)]
pub enum Event {
    SetAmount(u32),
    StartPayment,
    AbortPayment,
    SendPayment, // TODO Add transaction payload to send to acquirer
    ConfirmSend,
    SetReceiptEmail(String),
    SendReceipt,
    ConfirmSendReceipt,
    CompletePayment,
}

#[derive(Default)]
pub struct Model {
    payment: Option<Payment>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewModel {
    screen: Screen,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Screen {
    Payment(Payment),
    // TODO Settings
}

#[effect(typegen)]
pub enum Effect {
    Render(RenderOperation),
    Delay(DelayOperation),
}

#[derive(Default)]
pub struct App;

impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Capabilities = ();
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        _caps: &Self::Capabilities,
    ) -> Command<Effect, Event> {
        match event {
            Event::SetAmount(amount) => match &model.payment {
                Some(payment) if payment.status == PaymentStatus::New => {
                    model.payment = Some(Payment::new(amount));
                    render()
                }
                Some(_) => Command::done(),
                None => {
                    model.payment = Some(Payment::new(amount));
                    render()
                }
            },
            Event::StartPayment => {
                if let Some(payment) = &mut model.payment {
                    payment.start();
                };
                render()
            }
            Event::AbortPayment => {
                if let Some(Payment {
                    status: PaymentStatus::PendingTap,
                    ..
                }) = &model.payment
                {
                    model.payment = None;
                    render()
                } else {
                    Command::done()
                }
            }
            Event::SendPayment => {
                let mut commands = Vec::new();
                if let Some(payment) = &mut model.payment {
                    payment.send();

                    // Simulate processing delay
                    commands.push(Delay::start(2500).then_send(|()| Event::ConfirmSend));
                }
                commands.push(render());
                Command::all(commands)
            }
            Event::ConfirmSend => {
                if let Some(payment) = &mut model.payment {
                    payment.confirm_send();
                    render()
                } else {
                    Command::done()
                }
            }
            Event::CompletePayment => {
                if let Some(Payment {
                    status: PaymentStatus::Completed(_),
                    ..
                }) = &model.payment
                {
                    model.payment = None;
                    render()
                } else {
                    Command::done()
                }
            }
            Event::SetReceiptEmail(email) => {
                if let Some(payment) = &mut model.payment {
                    if let Some(receipt) = payment.receipt() {
                        receipt.email = email;
                    }
                };
                render()
            }
            Event::SendReceipt => {
                let mut commands = Vec::new();
                if let Some(payment) = &mut model.payment {
                    if let Some(receipt) = payment.receipt() {
                        receipt.send();

                        // Simulate processing delay
                        commands.push(Delay::start(1200).then_send(|()| Event::ConfirmSendReceipt));
                    }
                }
                commands.push(render());
                Command::all(commands)
            }
            Event::ConfirmSendReceipt => {
                if let Some(payment) = &mut model.payment {
                    if let Some(receipt) = payment.receipt() {
                        receipt.confirm_send();
                    }
                }
                render()
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        let payment = match &model.payment {
            Some(payment) => payment.clone(),
            None => Payment::default(),
        };

        ViewModel {
            screen: Screen::Payment(payment),
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_let_bind::assert_let;
    use crux_core::App as _;

    use crate::{App, Effect, Event, Model, Payment, PaymentStatus, Receipt, Screen, ViewModel};

    fn payment(amount: u32, status: PaymentStatus) -> Payment {
        Payment { amount, status }
    }

    #[test]
    fn starts_with_new_payment() {
        let app = App::default();
        let model = Model::default();

        let expected = ViewModel {
            screen: Screen::Payment(payment(0, PaymentStatus::New)),
        };
        let actual = app.view(&model);

        assert_eq!(actual, expected);
    }

    #[test]
    fn basic_happy_path_payment_journey() {
        let app = App::default();
        let mut model = Model::default();

        let _ = app.update(Event::SetAmount(1000), &mut model, &());
        let view = app.view(&model);

        assert_eq!(
            view.screen,
            Screen::Payment(payment(1000, PaymentStatus::New))
        );

        let _ = app.update(Event::StartPayment, &mut model, &());
        let view = app.view(&model);

        assert_eq!(
            view.screen,
            Screen::Payment(payment(1000, PaymentStatus::PendingTap))
        );

        let mut cmd = app.update(Event::SendPayment, &mut model, &());
        assert_let!(Effect::Delay(request), &mut cmd.effects().next().unwrap());

        let view = app.view(&model);

        assert_eq!(
            view.screen,
            Screen::Payment(payment(1000, PaymentStatus::Sent))
        );

        // Time passed
        request.resolve(()).expect("should resolve");
        for event in cmd.events() {
            let _ = app.update(event, &mut model, &());
        }

        let view = app.view(&model);
        assert_eq!(
            view.screen,
            Screen::Payment(payment(1000, PaymentStatus::Completed(Receipt::default())))
        );

        let _ = app.update(
            Event::SetReceiptEmail("bob@fake.com".to_string()),
            &mut model,
            &(),
        );

        let view = app.view(&model);
        let expected_receipt = Receipt {
            email: "bob@fake.com".to_string(),
            status: crate::ReceiptStatus::New,
        };

        assert_eq!(
            view.screen,
            Screen::Payment(payment(1000, PaymentStatus::Completed(expected_receipt)))
        );

        let mut cmd = app.update(Event::SendReceipt, &mut model, &());
        assert_let!(Effect::Delay(request), &mut cmd.effects().next().unwrap());

        let view = app.view(&model);
        let expected_receipt = Receipt {
            email: "bob@fake.com".to_string(),
            status: crate::ReceiptStatus::Pending,
        };

        assert_eq!(
            view.screen,
            Screen::Payment(payment(1000, PaymentStatus::Completed(expected_receipt)))
        );

        // Time passed
        request.resolve(()).unwrap();
        for event in cmd.events() {
            let _ = app.update(event, &mut model, &());
        }

        let view = app.view(&model);
        let expected_receipt = Receipt {
            email: "bob@fake.com".to_string(),
            status: crate::ReceiptStatus::Sent,
        };

        assert_eq!(
            view.screen,
            Screen::Payment(payment(1000, PaymentStatus::Completed(expected_receipt)))
        );

        let _ = app.update(Event::CompletePayment, &mut model, &());
        let view = app.view(&model);

        assert_eq!(view.screen, Screen::Payment(Payment::default()));
    }

    #[test]
    fn does_not_start_payment_of_zero() {
        let app = App::default();
        let mut model = Model::default();

        let _ = app.update(Event::SetAmount(0), &mut model, &());
        let _ = app.update(Event::StartPayment, &mut model, &());

        let actual = app.view(&model);
        let expected = ViewModel {
            screen: Screen::Payment(payment(0, PaymentStatus::New)),
        };

        assert_eq!(actual, expected)
    }
}
