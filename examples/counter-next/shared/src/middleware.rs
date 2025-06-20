use std::{
    sync::mpsc::{Sender, channel},
    thread::spawn,
};

use crux_core::{Request, middleware::EffectMiddleware};
use rand::{
    Rng as _, SeedableRng, TryRngCore as _,
    rngs::{OsRng, StdRng},
};

use crate::capabilities::{RandomNumber, RandomNumberRequest};

#[allow(clippy::type_complexity)]
pub struct RngMiddleware {
    jobs_tx: Sender<(RandomNumberRequest, Box<dyn FnOnce(RandomNumber) + Send>)>,
}

impl RngMiddleware {
    pub fn new() -> Self {
        let (jobs_tx, jobs_rx) =
            channel::<(RandomNumberRequest, Box<dyn FnOnce(RandomNumber) + Send>)>();

        // Persistent background worker
        spawn(move || {
            let mut os_rng = OsRng;
            let mut rng = StdRng::seed_from_u64(os_rng.try_next_u64().expect("could not seed RNG"));

            while let Ok((RandomNumberRequest(from, to), callback)) = jobs_rx.recv() {
                #[allow(clippy::cast_sign_loss)]
                let top = (to - from) as usize;
                #[allow(clippy::cast_possible_wrap)]
                let out = rng.random_range(0..top) as isize + from;

                callback(RandomNumber(out));
            }
        });

        Self { jobs_tx }
    }
}

impl<Effect> EffectMiddleware<Effect> for RngMiddleware
where
    Effect: TryInto<Request<RandomNumberRequest>, Error = Effect>,
{
    type Op = RandomNumberRequest;

    fn try_process_effect_with(
        &self,
        effect: Effect,
        resolve_callback: impl FnOnce(Request<RandomNumberRequest>, RandomNumber) + Send + 'static,
    ) -> Result<(), Effect> {
        let rand_request @ Request {
            operation: RandomNumberRequest(_, _),
            ..
        } = effect.try_into()?;

        self.jobs_tx
            .send((
                rand_request.operation.clone(),
                Box::new(move |number| resolve_callback(rand_request, number)),
            ))
            .expect("Job failed to send to worker thread");

        Ok(())
    }
}
