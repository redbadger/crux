use crux_core::capability::{Capability, CapabilityContext, Operation};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TimerOperation {
    Start { id: u64, millis: usize },
    Cancel { id: u64 },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TimerOutput {
    Created { id: u64 },
    Finished { id: u64 },
}

impl Operation for TimerOperation {
    type Output = TimerOutput;
}

pub struct Timer<Event> {
    context: CapabilityContext<TimerOperation, Event>,
}

impl<Ev> Timer<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<TimerOperation, Ev>) -> Self {
        Self { context }
    }

    pub fn start<F>(&self, id: u64, millis: usize, make_event: F)
    where
        F: Fn(TimerOutput) -> Ev + Clone + Send + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();

            async move {
                let mut stream = context.stream_from_shell(TimerOperation::Start { id, millis });

                while let Some(output) = stream.next().await {
                    let make_event = make_event.clone();

                    context.update_app(make_event(output));
                }
            }
        })
    }

    pub fn cancel(&self, id: u64) {
        self.context.spawn({
            let context = self.context.clone();

            async move {
                context.notify_shell(TimerOperation::Cancel { id }).await;
            }
        })
    }
}

impl<Ef> Capability<Ef> for Timer<Ef> {
    type Operation = TimerOperation;
    type MappedSelf<MappedEv> = Timer<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ef + Send + Sync + Copy + 'static,
        Ef: 'static,
        NewEvent: 'static,
    {
        Timer::new(self.context.map_event(f))
    }
}
