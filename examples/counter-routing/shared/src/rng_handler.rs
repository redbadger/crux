use std::{
    sync::{
        Weak,
        mpsc::{Sender, channel},
    },
    thread::spawn,
};

use crux_core::{Request, effects::ResolveSink};
use rand::rngs::SysRng;
use rand::{RngExt, SeedableRng, TryRng as _, rngs::StdRng};

use crate::capabilities::{RandomNumber, RandomNumberRequest};

pub struct RngHandler {
    jobs_tx: Sender<Request<RandomNumberRequest>>,
}

impl RngHandler {
    pub fn new<R>(sink: Weak<R>) -> Self
    where
        R: ResolveSink<RandomNumberRequest> + Send + Sync + 'static,
    {
        let (jobs_tx, jobs_rx) = channel::<Request<RandomNumberRequest>>();

        // Persistent background worker
        spawn(move || {
            let mut sys_rng = SysRng;
            let mut rng =
                StdRng::seed_from_u64(sys_rng.try_next_u64().expect("could not seed RNG"));

            while let Ok(request) = jobs_rx.recv() {
                let RandomNumberRequest(from, to) = request.operation;

                #[allow(clippy::cast_sign_loss)]
                let top = (to - from) as usize;
                #[allow(clippy::cast_possible_wrap)]
                let out = rng.random_range(0..top) as isize + from;

                if let Some(sink) = sink.upgrade() {
                    sink.resolve_request(request, RandomNumber(out))
                        .expect("background file store resolve should succeed");
                }
            }
        });

        Self { jobs_tx }
    }

    pub fn process(&self, request: Request<RandomNumberRequest>) {
        self.jobs_tx
            .send(request)
            .expect("RngHandler worker disconnected");
    }
}
