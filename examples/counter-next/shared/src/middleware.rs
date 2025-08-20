use std::{
    sync::mpsc::{Sender, channel},
    thread::spawn,
};

use crux_core::{Request, RequestHandle, middleware::EffectMiddleware};
use rand::{
    Rng as _, SeedableRng, TryRngCore as _,
    rngs::{OsRng, StdRng},
};

use crate::capabilities::{RandomNumber, RandomNumberRequest};

#[allow(clippy::type_complexity)]
pub struct RngMiddleware {
    jobs_tx: Sender<(
        RandomNumberRequest,
        RequestHandle<RandomNumber>,
        Box<dyn Fn(RequestHandle<RandomNumber>, RandomNumber) + Send>,
    )>,
}

impl RngMiddleware {
    pub fn new() -> Self {
        let (jobs_tx, jobs_rx) = channel::<(
            RandomNumberRequest,
            RequestHandle<RandomNumber>,
            Box<dyn Fn(RequestHandle<RandomNumber>, RandomNumber) + Send>,
        )>();

        // Persistent background worker
        spawn(move || {
            let mut os_rng = OsRng;
            let mut rng = StdRng::seed_from_u64(os_rng.try_next_u64().expect("could not seed RNG"));

            while let Ok((RandomNumberRequest(from, to), handle, callback)) = jobs_rx.recv() {
                #[allow(clippy::cast_sign_loss)]
                let top = (to - from) as usize;
                #[allow(clippy::cast_possible_wrap)]
                let out = rng.random_range(0..top) as isize + from;

                callback(handle, RandomNumber(out));
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
        resolve_callback: impl Fn(RequestHandle<RandomNumber>, RandomNumber) + Send + 'static,
    ) -> Result<(), Effect> {
        let rand_request = effect.try_into()?;
        let (operation, handle): (RandomNumberRequest, _) = rand_request.split();

        self.jobs_tx
            .send((operation, handle, Box::new(resolve_callback)))
            .expect("Job failed to send to worker thread");

        Ok(())
    }
}
