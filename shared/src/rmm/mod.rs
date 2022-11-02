mod capability;
mod cmd;
mod http;
mod key_value;
mod platform;
mod time;

pub use cmd::*;
pub use key_value::KeyValue;
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
            Response::Http { data } => {
                let msg = self.cmd.http.response(data);

                self.app.update(msg, &mut model, &self.cmd)
            }
            Response::Time { data } => {
                let msg = self.cmd.time.response(data);

                self.app.update(msg, &mut model, &self.cmd)
            }
            Response::Platform { data } => {
                let msg = self.cmd.platform.response(data);

                self.app.update(msg, &mut model, &self.cmd)
            }
            Response::KVRead { data } => {
                let msg = self.cmd.key_value_read.response(data);

                self.app.update(msg, &mut model, &self.cmd)
            }
            Response::KVWrite { data } => {
                let msg = self.cmd.key_value_write.response(data);

                self.app.update(msg, &mut model, &self.cmd)
            }
        }
    }

    pub fn view(&self) -> A::ViewModel {
        let model = self.model.read().unwrap();

        self.app.view(&model)
    }
}
