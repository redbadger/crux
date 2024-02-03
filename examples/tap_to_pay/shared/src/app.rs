mod payment;

use crux_core::render::Render;
use serde::{Deserialize, Serialize};

use crate::capabilities::delay::Delay;
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

#[derive(crux_core::macros::Effect)]
#[effect(app = "App")]
#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
pub struct Capabilities {
    render: Render<Event>,
    delay: Delay<Event>,
}

#[derive(Default)]
pub struct App;

impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Capabilities = Capabilities;

    fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) {
        match event {
            Event::SetAmount(amount) => {
                match &model.payment {
                    Some(payment) if payment.status == PaymentStatus::New => {
                        model.payment = Some(Payment::new(amount))
                    }
                    Some(_) => (),
                    None => model.payment = Some(Payment::new(amount)),
                };
            }
            Event::StartPayment => {
                if let Some(payment) = &mut model.payment {
                    payment.start()
                }
            }
            Event::AbortPayment => {
                if let Some(Payment {
                    status: PaymentStatus::PendingTap,
                    ..
                }) = &model.payment
                {
                    model.payment = None
                }
            }
            Event::SendPayment => {
                if let Some(payment) = &mut model.payment {
                    payment.send();

                    // Simulate processing delay
                    caps.delay.start(2500, Event::ConfirmSend)
                }
            }
            Event::ConfirmSend => {
                if let Some(payment) = &mut model.payment {
                    payment.confirm_send();
                }
            }
            Event::CompletePayment => {
                if let Some(Payment {
                    status: PaymentStatus::Completed(_),
                    ..
                }) = &model.payment
                {
                    model.payment = None
                }
            }
            Event::SetReceiptEmail(email) => {
                if let Some(payment) = &mut model.payment {
                    if let Some(receipt) = payment.receipt() {
                        receipt.email = email;
                    }
                }
            }
            Event::SendReceipt => {
                if let Some(payment) = &mut model.payment {
                    if let Some(receipt) = payment.receipt() {
                        receipt.send();

                        // Simulate processing delay
                        caps.delay.start(1200, Event::ConfirmSendReceipt)
                    }
                }
            }
            Event::ConfirmSendReceipt => {
                if let Some(payment) = &mut model.payment {
                    if let Some(receipt) = payment.receipt() {
                        receipt.confirm_send();
                    }
                }
            }
        };

        caps.render.render();
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
    use crux_core::testing::AppTester;

    use crate::{App, Effect, Event, Model, Payment, PaymentStatus, Receipt, Screen, ViewModel};

    fn payment(amount: u32, status: PaymentStatus) -> Payment {
        Payment { amount, status }
    }

    #[test]
    fn starts_with_new_payment() {
        let app = AppTester::<App, _>::default();
        let model = Model::default();

        let expected = ViewModel {
            screen: Screen::Payment(payment(0, PaymentStatus::New)),
        };
        let actual = app.view(&model);

        assert_eq!(actual, expected);
    }

    #[test]
    fn basic_happy_path_payment_journey() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        app.update(Event::SetAmount(1000), &mut model);
        let view = app.view(&model);

        assert_eq!(
            view.screen,
            Screen::Payment(payment(1000, PaymentStatus::New))
        );

        app.update(Event::StartPayment, &mut model);
        let view = app.view(&model);

        assert_eq!(
            view.screen,
            Screen::Payment(payment(1000, PaymentStatus::PendingTap))
        );

        let mut update = app.update(Event::SendPayment, &mut model);
        assert_let!(Effect::Delay(request), &mut update.effects[0]);

        let view = app.view(&model);

        assert_eq!(
            view.screen,
            Screen::Payment(payment(1000, PaymentStatus::Sent))
        );

        // Time passed
        let update = app.resolve(request, ()).expect("should resolve");
        for event in update.events {
            app.update(event, &mut model);
        }

        let view = app.view(&model);
        assert_eq!(
            view.screen,
            Screen::Payment(payment(1000, PaymentStatus::Completed(Receipt::default())))
        );

        app.update(
            Event::SetReceiptEmail("bob@fake.com".to_string()),
            &mut model,
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

        let mut update = app.update(Event::SendReceipt, &mut model);
        assert_let!(Effect::Delay(request), &mut update.effects[0]);

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
        let update = app.resolve(request, ()).expect("should update");
        for event in update.events {
            app.update(event, &mut model);
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

        app.update(Event::CompletePayment, &mut model);
        let view = app.view(&model);

        assert_eq!(view.screen, Screen::Payment(Payment::default()));
    }

    #[test]
    fn does_not_start_payment_of_zero() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        app.update(Event::SetAmount(0), &mut model);
        app.update(Event::StartPayment, &mut model);

        let actual = app.view(&model);
        let expected = ViewModel {
            screen: Screen::Payment(payment(0, PaymentStatus::New)),
        };

        assert_eq!(actual, expected)
    }
}
