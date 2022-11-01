mod cmd;
mod http;
mod key_value;
mod platform;
mod time;

pub use cmd::{Cmd, Request, Response};
use std::sync::RwLock;

pub trait App: Default {
    type Msg;
    type Model: Default;
    type ViewModel;

    fn update(
        &self,
        msg: <Self as App>::Msg,
        model: &mut <Self as App>::Model,
        cmd: &Cmd<<Self as App>::Msg>,
    ) -> Vec<Request>;

    fn view(&self, model: &<Self as App>::Model) -> <Self as App>::ViewModel;
}

pub struct AppCore<A: App> {
    model: RwLock<A::Model>,
    cmd: Cmd<A::Msg>,
    app: A,
}

impl<A: App> PartialEq for AppCore<A> {
    fn eq(&self, _other: &Self) -> bool {
        false // Core has all kinds of interior mutability
    }
}

impl<A: App> Default for AppCore<A> {
    fn default() -> Self {
        Self {
            model: Default::default(),
            cmd: Default::default(),
            app: Default::default(),
        }
    }
}

impl<A: App> AppCore<A> {
    pub fn new() -> Self {
        Self::default()
    }

    // Direct message
    pub fn message(&self, msg: A::Msg) -> Vec<Request> {
        let mut model = self.model.write().unwrap();

        self.app.update(msg, &mut model, &self.cmd)
    }

    // Return from capability
    pub fn response(&self, res: Response) -> Vec<Request> {
        let mut model = self.model.write().unwrap();
        match res {
            Response::Http { uuid, bytes } => {
                let msg = self.cmd.http.receive(&uuid, bytes);

                self.app.update(msg, &mut model, &self.cmd)
            }
            Response::Time { uuid, iso_time } => {
                let msg = self.cmd.time.receive(&uuid, iso_time);

                self.app.update(msg, &mut model, &self.cmd)
            }
            Response::Platform { uuid, platform } => {
                let msg = self.cmd.platform.receive(&uuid, platform);

                self.app.update(msg, &mut model, &self.cmd)
            }
            Response::KVRead { uuid, bytes } => {
                let msg = self.cmd.key_value.receive_read(&uuid, bytes);

                self.app.update(msg, &mut model, &self.cmd)
            }
            Response::KVWrite { uuid, success } => {
                let msg = self.cmd.key_value.written(&uuid, success);

                self.app.update(msg, &mut model, &self.cmd)
            }
        }
    }

    pub fn view(&self) -> A::ViewModel {
        let model = self.model.read().unwrap();

        self.app.view(&model)
    }
}
