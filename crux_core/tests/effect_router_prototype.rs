use std::sync::{Arc, Mutex};

use crux_time::{TimeRequest, TimeResponse};

use crate::ffi::FfiEffect;

mod app {
    use crux_core::capability::Operation;
    use crux_macros::effect;
    use serde::{Deserialize, Serialize};
    use std::time::Duration;

    use crux_core::App;
    use crux_core::Command;
    use crux_core::render::RenderOperation;
    use crux_core::render::render;
    use crux_time::{Time, TimeRequest, TimerOutcome};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CaptureImage;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpaqueImageRef(pub usize);

    impl Operation for CaptureImage {
        type Output = OpaqueImageRef;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct StoreFile {
        pub(crate) image: OpaqueImageRef,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct StoredImage {
        pub(crate) path: String,
    }

    impl Operation for StoreFile {
        type Output = StoredImage;
    }

    #[effect]
    #[derive(Debug)]
    pub(crate) enum Effect {
        Render(RenderOperation),
        Time(TimeRequest),
        Camera(CaptureImage),
        FileStore(StoreFile),
    }

    #[derive(Debug)]
    pub(crate) enum Event {
        Trigger,
        TriggerWithTimer,

        TimerFired(TimerOutcome),
        CameraCaptured(OpaqueImageRef),
        FileStored(StoredImage),
    }

    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    pub(crate) enum SaveStatus {
        #[default]
        Idle,
        WaitingForCapture,
        Saving,
        Saved,
    }

    #[derive(Debug, Clone, Default)]
    pub(crate) struct Model {
        pub(crate) countdown: Option<u8>,
        pub(crate) save_status: SaveStatus,
        pub(crate) saved_path: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct ViewModel {
        pub(crate) countdown: Option<u8>,
        pub(crate) save_status: SaveStatus,
        pub(crate) saved_path: Option<String>,
    }

    #[derive(Default)]
    pub(crate) struct SelfieApp;

    impl SelfieApp {
        pub(crate) fn one_second_timer() -> Command<Effect, Event> {
            let (timer, _handle) = Time::<Effect, Event>::notify_after(Duration::from_secs(1));

            timer.then_send(Event::TimerFired)
        }

        pub(crate) fn request_camera() -> Command<Effect, Event> {
            Command::request_from_shell(CaptureImage).then_send(Event::CameraCaptured)
        }
    }

    impl App for SelfieApp {
        type Event = Event;
        type Model = Model;
        type ViewModel = ViewModel;
        type Effect = Effect;

        fn update(
            &self,
            event: Self::Event,
            model: &mut Self::Model,
        ) -> Command<Self::Effect, Self::Event> {
            match event {
                Event::Trigger => {
                    model.countdown = None;
                    model.save_status = SaveStatus::WaitingForCapture;

                    Command::all([Self::request_camera(), render()])
                }
                Event::TriggerWithTimer => {
                    model.countdown = Some(3);
                    model.save_status = SaveStatus::WaitingForCapture;

                    Command::all([Self::one_second_timer(), render()])
                }
                Event::TimerFired(TimerOutcome::Completed(_)) => {
                    let Some(countdown) = model.countdown else {
                        return Command::done();
                    };

                    if countdown > 1 {
                        model.countdown = Some(countdown - 1);

                        Command::all([Self::one_second_timer(), render()])
                    } else {
                        model.countdown = None;

                        Command::all([Self::request_camera(), render()])
                    }
                }
                Event::TimerFired(TimerOutcome::Cleared) => Command::done(),
                Event::CameraCaptured(image) => {
                    model.save_status = SaveStatus::Saving;

                    let store = Command::request_from_shell(StoreFile { image })
                        .then_send(Event::FileStored);

                    Command::all([store, render()])
                }
                Event::FileStored(stored) => {
                    model.save_status = SaveStatus::Saved;
                    model.saved_path = Some(stored.path);

                    render()
                }
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            ViewModel {
                countdown: model.countdown,
                save_status: model.save_status.clone(),
                saved_path: model.saved_path.clone(),
            }
        }
    }
}

// A version of this becomes part of crux_core
mod crux_provided {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicU32;
    use std::sync::atomic::Ordering;

    use crux_core::Core;
    use crux_core::Request;
    use crux_core::Resolvable;
    use crux_core::capability::Operation;
    use crux_core::{RequestHandle, ResolveError};

    use crate::app;

    pub(crate) struct Registry<Op: Operation> {
        pub(crate) next_id: AtomicU32,
        pub(crate) requests: Mutex<HashMap<u32, Request<Op>>>,
    }

    impl<Op: Operation> Default for Registry<Op> {
        fn default() -> Self {
            Self {
                next_id: AtomicU32::new(0),
                requests: Mutex::new(HashMap::new()),
            }
        }
    }

    impl<Op> Registry<Op>
    where
        Op: Operation + Clone,
    {
        pub(crate) fn register(&self, request: Request<Op>) -> (u32, Op) {
            let id = self.next_id.fetch_add(1, Ordering::Relaxed);
            let operation = request.operation.clone();

            self.requests
                .lock()
                .expect("registry lock poisoned")
                .insert(id, request);

            (id, operation)
        }
    }

    impl<Op> Registry<Op>
    where
        Op: Operation,
    {
        pub(crate) fn resolve_with<F>(
            &self,
            id: u32,
            output: Op::Output,
            resolve: F,
        ) -> Result<(), ResolveError>
        where
            F: FnOnce(&mut Request<Op>, Op::Output) -> Result<(), ResolveError>,
        {
            let mut requests = self.requests.lock().expect("registry lock poisoned");
            let Some(mut request) = requests.remove(&id) else {
                panic!("missing pending handle for id {id}");
            };
            drop(requests);

            resolve(&mut request, output)?;

            if !matches!(request.handle, RequestHandle::Never) {
                self.requests
                    .lock()
                    .expect("registry lock poisoned")
                    .insert(id, request);
            }

            Ok(())
        }
    }

    pub(crate) struct Router {
        core: Core<app::SelfieApp>,
        route_effects: Box<dyn Fn(Vec<app::Effect>) + Send + Sync>,
    }

    impl Router {
        pub(crate) fn new(
            core: Core<app::SelfieApp>,
            route_effects: impl Fn(Vec<app::Effect>) + Send + Sync + 'static,
        ) -> Self {
            Self {
                core,
                route_effects: Box::new(route_effects),
            }
        }

        pub(crate) fn update(&self, event: app::Event) {
            let effects = self.core.process_event(event);
            (self.route_effects)(effects);
        }

        pub(crate) fn resolve<Output>(
            &self,
            request: &mut impl Resolvable<Output>,
            output: Output,
        ) -> Result<(), ResolveError> {
            let effects = self.core.resolve(request, output)?;
            (self.route_effects)(effects);
            Ok(())
        }

        pub(crate) fn view(&self) -> app::ViewModel {
            self.core.view()
        }
    }
}

mod ffi {
    use std::sync::{Arc, OnceLock};

    use crux_core::{Core, Request, RequestHandle};
    use crux_time::TimeRequest;
    use crux_time::TimeResponse;
    use serde::{Deserialize, Serialize};

    use super::app;
    use super::crux_provided::{self, Router};

    // Effect types for the shell

    /// A narrowed down effect with only Time and Render
    ///
    /// Used by the serialized channel
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) enum FfiEffect {
        Render,
        Time { id: u32, request: TimeRequest },
    }

    /// A specific typed effect with opaque or pointer based payload
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) struct CameraEffect {
        pub(crate) id: u32,
        pub(crate) operation: app::CaptureImage,
    }

    // Trait implemented by the shells, e.g with uniffi of wasm_bindgen
    pub(crate) trait CameraShell: Send + Sync {
        fn process_serialized_effects(&self, bytes: Vec<u8>);
        fn process_camera_effect(&self, effect: CameraEffect);
    }

    pub(crate) struct CameraFFI {
        router: Arc<Router>,
        serialized_registry: Arc<crux_provided::Registry<TimeRequest>>,
        camera_registry: Arc<crux_provided::Registry<app::CaptureImage>>,
    }

    fn emit_serialized_effect(shell: &dyn CameraShell, effect: FfiEffect) {
        let bytes = serde_json::to_vec(&vec![effect]).expect("serialized effect should encode");
        shell.process_serialized_effects(bytes);
    }

    /// The core FFI
    impl CameraFFI {
        pub(crate) fn new(shell: Arc<dyn CameraShell>) -> Self {
            let router_ref = Arc::new(OnceLock::<Arc<Router>>::new());
            let serialized_registry = Arc::new(crux_provided::Registry::default());
            let camera_registry = Arc::new(crux_provided::Registry::default());

            let file_store = Arc::new(file_store::AsyncFileStoreHandler::new({
                let router_ref = router_ref.clone();
                move |mut request: Request<app::StoreFile>, stored: app::StoredImage| {
                    let router = router_ref.get().expect("router should be initialized");
                    router
                        .resolve(&mut request, stored)
                        .expect("background file store resolve should succeed");
                }
            }));

            let shell_for_route = shell.clone();
            let serialized_registry_for_route = serialized_registry.clone();
            let camera_registry_for_route = camera_registry.clone();
            let file_store_for_route = file_store.clone();
            let router = Arc::new(Router::new(Core::new(), move |effects| {
                for effect in effects {
                    match effect {
                        app::Effect::Render(request) => {
                            let (_op, handle) = request.split();
                            assert!(matches!(handle, RequestHandle::Never));
                            emit_serialized_effect(shell_for_route.as_ref(), FfiEffect::Render);
                        }
                        app::Effect::Time(request) => {
                            let (id, op) = serialized_registry_for_route.register(request);
                            emit_serialized_effect(
                                shell_for_route.as_ref(),
                                FfiEffect::Time { id, request: op },
                            );
                        }
                        app::Effect::Camera(request) => {
                            let (id, op) = camera_registry_for_route.register(request);
                            shell_for_route
                                .process_camera_effect(CameraEffect { id, operation: op });
                        }
                        app::Effect::FileStore(request) => {
                            file_store_for_route.process_file_store(request);
                        }
                    }
                }
            }));

            let _ = router_ref.set(router.clone());

            Self {
                router,
                serialized_registry,
                camera_registry,
            }
        }

        pub(crate) fn update(&self, event: app::Event) {
            self.router.update(event);
        }

        /// Resolve an effect sent over the serialized lane.
        pub(crate) fn resolve(&self, id: u32, data: &[u8]) {
            // FIXME: use the existing Bridge
            let output: TimeResponse =
                serde_json::from_slice(data).expect("serialized resolve payload should decode");

            self.serialized_registry
                .resolve_with(id, output, |request, output| {
                    self.router.resolve(request, output)
                })
                .expect("serialized resolve should work");
        }

        /// Specific effect FFI for opaque data types
        pub(crate) fn resolve_camera(&self, id: u32, output: app::OpaqueImageRef) {
            self.camera_registry
                .resolve_with(id, output, |request, output| {
                    self.router.resolve(request, output)
                })
                .expect("camera resolve should work");
        }

        pub(crate) fn view(&self) -> app::ViewModel {
            self.router.view()
        }
    }

    // Example core-side effect handler implementation
    mod file_store {
        use std::sync::Arc;
        use std::thread;
        use std::time::Duration;

        use crossbeam_channel::{Receiver, Sender, unbounded};
        use crux_core::Request;

        use super::app;

        pub(crate) struct AsyncFileStoreHandler {
            jobs_tx: Sender<Request<app::StoreFile>>,
        }

        impl AsyncFileStoreHandler {
            pub(crate) fn new(
                resolve: impl Fn(Request<app::StoreFile>, app::StoredImage) + Send + Sync + 'static,
            ) -> Self {
                let (jobs_tx, jobs_rx) = unbounded();
                let resolve = Arc::new(resolve);

                thread::spawn(move || worker(jobs_rx, resolve));

                Self { jobs_tx }
            }

            pub(crate) fn process_file_store(&self, request: Request<app::StoreFile>) {
                self.jobs_tx
                    .send(request)
                    .expect("file store worker queue disconnected")
            }
        }

        fn worker(
            jobs_rx: Receiver<Request<app::StoreFile>>,
            resolve: Arc<dyn Fn(Request<app::StoreFile>, app::StoredImage) + Send + Sync>,
        ) {
            while let Ok(request) = jobs_rx.recv() {
                // Ensure async behavior so this path models background work.
                thread::sleep(Duration::from_millis(1));

                let image_id = request.operation.image.0;
                let stored = app::StoredImage {
                    path: format!("/tmp/selfie-{image_id}.jpg"),
                };

                resolve(request, stored);
            }
        }
    }
}

/// Pretend shell for testing purposes, implements CameraShell like a real shell would via uniffi or wasm_bindgen
/// This is only a test stub, IRL the shell would process the effect, then call the core with a result, in this case
/// the tests do the equivalent
#[derive(Default)]
pub(crate) struct TestShell {
    pub(crate) serialized: Mutex<Vec<Vec<u8>>>,
    pub(crate) camera: Mutex<Vec<ffi::CameraEffect>>,
}

impl ffi::CameraShell for TestShell {
    fn process_serialized_effects(&self, bytes: Vec<u8>) {
        self.serialized
            .lock()
            .expect("fake shell serialized lock poisoned")
            .push(bytes);
    }

    fn process_camera_effect(&self, effect: ffi::CameraEffect) {
        self.camera
            .lock()
            .expect("fake shell camera lock poisoned")
            .push(effect);
    }
}

impl TestShell {
    /// Test-only helper: take serialized effects emitted from the core
    pub(crate) fn take_serialized_effects(&self) -> Vec<ffi::FfiEffect> {
        let payloads = self
            .serialized
            .lock()
            .expect("fake shell serialized lock poisoned")
            .drain(..)
            .collect::<Vec<_>>();

        payloads
            .into_iter()
            .flat_map(|bytes| {
                serde_json::from_slice::<Vec<ffi::FfiEffect>>(&bytes)
                    .expect("serialized shell payload should decode")
            })
            .collect()
    }

    /// Test only helper: take camera effects emitted from the core
    pub(crate) fn take_camera_effects(&self) -> Vec<ffi::CameraEffect> {
        self.camera
            .lock()
            .expect("fake shell camera lock poisoned")
            .drain(..)
            .collect()
    }
}

#[test]
fn trigger_takes_a_picture() {
    let shell = Arc::new(TestShell::default());
    let core = ffi::CameraFFI::new(shell.clone());

    core.update(app::Event::Trigger);

    let camera_effects = shell.take_camera_effects();
    assert_eq!(camera_effects.len(), 1);
    assert_eq!(camera_effects[0].operation, app::CaptureImage);

    let serialized = shell.take_serialized_effects();
    assert_eq!(serialized, vec![ffi::FfiEffect::Render]);

    assert_eq!(
        core.view(),
        app::ViewModel {
            countdown: None,
            save_status: app::SaveStatus::WaitingForCapture,
            saved_path: None,
        }
    );
}

#[test]
fn trigger_with_timer_takes_a_picture_after_countdown() {
    let shell = Arc::new(TestShell::default());
    let core = ffi::CameraFFI::new(shell.clone());

    core.update(app::Event::TriggerWithTimer);

    assert_eq!(
        core.view(),
        app::ViewModel {
            countdown: Some(3),
            save_status: app::SaveStatus::WaitingForCapture,
            saved_path: None,
        }
    );

    let mut serialized = shell.take_serialized_effects();
    assert!(serialized.contains(&ffi::FfiEffect::Render));

    // 3 -> 2
    let (request_id, request) = extract_single_time_request(std::mem::take(&mut serialized));
    let TimeRequest::NotifyAfter { id: timer_id, .. } = request else {
        panic!("expected NotifyAfter request")
    };
    let response = TimeResponse::DurationElapsed { id: timer_id };
    let response_bytes = serde_json::to_vec(&response).expect("time response should serialize");
    core.resolve(request_id, &response_bytes);

    assert_eq!(core.view().countdown, Some(2));
    serialized = shell.take_serialized_effects();
    assert!(serialized.contains(&ffi::FfiEffect::Render));

    // 2 -> 1
    let (request_id, request) = extract_single_time_request(std::mem::take(&mut serialized));
    let TimeRequest::NotifyAfter { id: timer_id, .. } = request else {
        panic!("expected NotifyAfter request")
    };
    let response = TimeResponse::DurationElapsed { id: timer_id };
    let response_bytes = serde_json::to_vec(&response).expect("time response should serialize");
    core.resolve(request_id, &response_bytes);

    assert_eq!(core.view().countdown, Some(1));
    serialized = shell.take_serialized_effects();
    assert!(serialized.contains(&ffi::FfiEffect::Render));

    // 1 -> camera capture
    let (request_id, request) = extract_single_time_request(std::mem::take(&mut serialized));
    let TimeRequest::NotifyAfter { id: timer_id, .. } = request else {
        panic!("expected NotifyAfter request")
    };
    let response = TimeResponse::DurationElapsed { id: timer_id };
    let response_bytes = serde_json::to_vec(&response).expect("time response should serialize");
    core.resolve(request_id, &response_bytes);

    assert_eq!(core.view().countdown, None);

    let camera_effects = shell.take_camera_effects();
    assert_eq!(camera_effects.len(), 1);
    assert_eq!(camera_effects[0].operation, app::CaptureImage);

    let serialized = shell.take_serialized_effects();
    assert_eq!(serialized, vec![ffi::FfiEffect::Render]);
}

#[test]
fn pictures_are_saved_to_file_system() {
    use std::thread;
    use std::time::Duration;

    let shell = Arc::new(TestShell::default());
    let core = ffi::CameraFFI::new(shell.clone());

    core.update(app::Event::Trigger);

    let camera_effect = shell.take_camera_effects().remove(0);

    // Drain render from initial trigger; focus assertions on the save flow.
    let _ = shell.take_serialized_effects();

    core.resolve_camera(camera_effect.id, app::OpaqueImageRef(42));

    assert_eq!(
        core.view(),
        app::ViewModel {
            countdown: None,
            save_status: app::SaveStatus::Saving,
            saved_path: None,
        }
    );

    let serialized = shell.take_serialized_effects();
    assert_eq!(serialized, vec![ffi::FfiEffect::Render]);

    for _ in 0..50 {
        if core.view().save_status == app::SaveStatus::Saved {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }

    assert_eq!(
        core.view(),
        app::ViewModel {
            countdown: None,
            save_status: app::SaveStatus::Saved,
            saved_path: Some("/tmp/selfie-42.jpg".to_string()),
        }
    );

    let serialized = shell.take_serialized_effects();
    assert_eq!(serialized, vec![ffi::FfiEffect::Render]);
}

pub(crate) fn extract_single_time_request(effects: Vec<ffi::FfiEffect>) -> (u32, TimeRequest) {
    let mut time_effects: Vec<_> = effects
        .into_iter()
        .filter_map(|effect| match effect {
            ffi::FfiEffect::Time { id, request } => Some((id, request)),
            FfiEffect::Render => None,
        })
        .collect();

    assert_eq!(time_effects.len(), 1, "expected exactly one time request");

    time_effects.remove(0)
}
