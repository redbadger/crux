use crux_core::{platform, App, Command};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct Platform;

#[derive(Default, Serialize, Deserialize)]
pub struct Model {
    pub platform: String,
}

#[derive(Serialize, Deserialize)]
pub enum PlatformMsg {
    Get,
    Set(String),
}

impl App for Platform {
    type Message = PlatformMsg;
    type Model = Model;
    type ViewModel = Model;

    fn update(
        &self,
        msg: <Self as App>::Message,
        model: &mut <Self as App>::Model,
    ) -> Vec<Command<PlatformMsg>> {
        match msg {
            PlatformMsg::Get => vec![platform::get(Box::new(PlatformMsg::Set))],
            PlatformMsg::Set(platform) => {
                model.platform = platform;
                vec![Command::render()]
            }
        }
    }

    fn view(&self, model: &<Self as App>::Model) -> <Self as App>::ViewModel {
        Model {
            platform: model.platform.clone(),
        }
    }
}
