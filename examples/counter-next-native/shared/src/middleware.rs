use std::{
    sync::mpsc::{channel, Sender},
    thread::spawn,
};

use crux_core::middleware::{EffectMiddleware, EffectResolver};
use rand::{
    rngs::{OsRng, StdRng},
    Rng as _, SeedableRng, TryRngCore as _,
};

use crate::capabilities::{RandomNumber, RandomNumberRequest};

#[allow(clippy::type_complexity)]
pub struct RngMiddleware {
    jobs_tx: Sender<(RandomNumberRequest, EffectResolver<RandomNumber>)>,
}

impl RngMiddleware {
    pub fn new() -> Self {
        let (jobs_tx, jobs_rx) = channel::<(RandomNumberRequest, EffectResolver<RandomNumber>)>();

        // Persistent background worker
        spawn(move || {
            let mut os_rng = OsRng;
            let mut rng = StdRng::seed_from_u64(os_rng.try_next_u64().expect("could not seed RNG"));

            while let Ok((RandomNumberRequest(from, to), mut resolver)) = jobs_rx.recv() {
                #[allow(clippy::cast_sign_loss)]
                let top = (to - from) as usize;
                #[allow(clippy::cast_possible_wrap)]
                let out = rng.random_range(0..top) as isize + from;

                resolver.resolve(RandomNumber(out));
            }
        });

        Self { jobs_tx }
    }
}

impl EffectMiddleware for RngMiddleware {
    type Op = RandomNumberRequest;

    fn try_process_effect(
        &self,
        operation: RandomNumberRequest,
        resolver: EffectResolver<RandomNumber>,
    ) {
        self.jobs_tx
            .send((operation, resolver))
            .expect("Job failed to send to worker thread");
    }
}
