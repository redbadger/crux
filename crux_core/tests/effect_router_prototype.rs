use crux_time::{TimeRequest, TimeResponse};
use std::sync::{Arc, Mutex};

use crate::{
    app::{
        CaptureImageOp, Event, OpaqueImageRef, PermissionRequest, PermissionResponse, SaveStatus,
        ViewModel,
    },
    ffi::{CameraEffect, CameraFFI, FfiEffect, FfiRequest},
};

mod app {
    use crux_core::{
        App, Command,
        capability::Operation,
        render::{RenderOperation, render},
    };
    use crux_macros::effect;
    use crux_time::{Time, TimeRequest, TimerOutcome};
    use serde::{Deserialize, Serialize};
    use std::time::Duration;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CaptureImageOp;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpaqueImageRef(pub usize);

    impl Operation for CaptureImageOp {
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

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum PermissionRequest {
        Camera,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum PermissionResponse {
        Granted,
        Denied,
    }

    impl Operation for PermissionRequest {
        type Output = PermissionResponse;
    }

    #[effect]
    #[derive(Debug)]
    pub(crate) enum Effect {
        Render(RenderOperation),
        Time(TimeRequest),
        Permission(PermissionRequest),
        Camera(CaptureImageOp),
        FileStore(StoreFile),
    }

    #[derive(Debug)]
    pub(crate) enum Event {
        Trigger,
        TriggerWithTimer,

        PermissionForTrigger(PermissionResponse),
        PermissionForTimer(PermissionResponse),
        TimerFired(TimerOutcome),
        CameraCaptured(OpaqueImageRef),
        FileStored(StoredImage),
    }

    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    pub(crate) enum SaveStatus {
        #[default]
        Idle,
        WaitingForPermission,
        WaitingForCapture,
        Saving,
        Saved,
        PermissionDenied,
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
            let (timer, _handle) = Time::notify_after(Duration::from_secs(1));

            timer.then_send(Event::TimerFired)
        }

        pub(crate) fn request_permission_for_trigger() -> Command<Effect, Event> {
            Command::request_from_shell(PermissionRequest::Camera)
                .then_send(Event::PermissionForTrigger)
        }

        pub(crate) fn request_permission_for_timer() -> Command<Effect, Event> {
            Command::request_from_shell(PermissionRequest::Camera)
                .then_send(Event::PermissionForTimer)
        }

        pub(crate) fn request_camera() -> Command<Effect, Event> {
            Command::request_from_shell(CaptureImageOp).then_send(Event::CameraCaptured)
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
                    model.save_status = SaveStatus::WaitingForPermission;

                    Command::all([Self::request_permission_for_trigger(), render()])
                }
                Event::TriggerWithTimer => {
                    model.countdown = None;
                    model.save_status = SaveStatus::WaitingForPermission;

                    Command::all([Self::request_permission_for_timer(), render()])
                }
                Event::PermissionForTrigger(PermissionResponse::Granted) => {
                    model.save_status = SaveStatus::WaitingForCapture;

                    Command::all([Self::request_camera(), render()])
                }
                Event::PermissionForTimer(PermissionResponse::Granted) => {
                    model.countdown = Some(3);
                    model.save_status = SaveStatus::WaitingForCapture;

                    Command::all([Self::one_second_timer(), render()])
                }
                Event::PermissionForTrigger(PermissionResponse::Denied)
                | Event::PermissionForTimer(PermissionResponse::Denied) => {
                    model.save_status = SaveStatus::PermissionDenied;

                    render()
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

mod ffi {
    use crux_core::{
        Core,
        bridge::{EffectId as BridgeEffectId, FfiFormat, JsonFfiFormat, Request as BridgeRequest},
        effects::{
            EffectId, EffectRouter, Routes,
            routes::{Parked, Serialized},
        },
        render::RenderOperation,
    };
    use crux_macros::effect;
    use crux_time::TimeRequest;
    use std::sync::{Arc, Weak};

    use crate::{
        app::{CaptureImageOp, Effect, Event, OpaqueImageRef, PermissionRequest, SelfieApp},
        ffi::file_store::FileStoreHandler,
    };

    use super::app;

    /// Narrowed effect, down to the serialized variants
    ///
    /// This is only necessary if some variants are 'special'
    #[effect(typegen)]
    #[derive(Debug)]
    pub enum SerializedEffect {
        Render(RenderOperation),
        Time(TimeRequest),
        Permission(PermissionRequest),
    }

    impl TryFrom<Effect> for SerializedEffect {
        type Error = Effect;

        fn try_from(value: Effect) -> Result<Self, Self::Error> {
            match value {
                Effect::Render(request) => Ok(Self::Render(request)),
                Effect::Time(request) => Ok(Self::Time(request)),
                Effect::Permission(request) => Ok(Self::Permission(request)),
                other => Err(other),
            }
        }
    }

    pub(crate) type FfiEffect = SerializedEffectFfi;
    pub(crate) type FfiRequest = BridgeRequest<SerializedEffectFfi>;

    /// A specific typed effect with opaque (e.g. pointer based) payload
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) struct CameraEffect {
        pub(crate) id: u64,
        pub(crate) operation: CaptureImageOp,
    }

    // Trait implemented by the shells, e.g with UniFFI of wasm_bindgen
    pub(crate) trait CameraShell: Send + Sync {
        fn process_serialized_effects(&self, bytes: Vec<u8>);
        fn process_camera_effect(&self, effect: CameraEffect);
    }

    pub(crate) struct CameraFFI<Format: FfiFormat = JsonFfiFormat> {
        router: Arc<EffectRouter<SelfieApp, EffectRoutes<Format>>>,
    }

    struct EffectRoutes<Format: FfiFormat> {
        serialized: Arc<Serialized<SelfieApp, Self, Format>>,
        camera: Arc<Parked<SelfieApp, Self, CaptureImageOp>>,
        file_store: Arc<FileStoreHandler>,
    }

    impl<Format: FfiFormat> Clone for EffectRoutes<Format> {
        fn clone(&self) -> Self {
            Self {
                serialized: self.serialized.clone(),
                camera: self.camera.clone(),
                file_store: self.file_store.clone(),
            }
        }
    }

    impl<Format: FfiFormat> Routes<SelfieApp> for EffectRoutes<Format> {
        fn new(router: Weak<EffectRouter<SelfieApp, Self>>) -> Self {
            Self {
                serialized: Arc::new(Serialized::new(router.clone())),
                camera: Arc::new(Parked::new(router.clone())),
                file_store: Arc::new(FileStoreHandler::new(router)),
            }
        }
    }

    /// The core FFI
    impl CameraFFI<JsonFfiFormat> {
        pub(crate) fn new(shell: Arc<dyn CameraShell>) -> Self {
            Self::new_with_format(shell)
        }
    }

    impl<Format: FfiFormat> CameraFFI<Format> {
        pub(crate) fn new_with_format(shell: Arc<dyn CameraShell>) -> Self {
            let core = Core::new();

            let router = EffectRouter::new(core, move |routes: EffectRoutes<Format>| {
                let routes = routes.clone();
                let shell = shell.clone();

                move |effect| match effect {
                    // Core-side effect, processed in Rust
                    Effect::FileStore(request) => {
                        routes.file_store.process_file_store(request);
                    }
                    // Shell-side effect, but with a custom FFI to allow for opaque types, pointers, etc.
                    // this doesn't necessarily need to rely on the shell instance either, could be another
                    // form of callback to the shell
                    Effect::Camera(request) => {
                        let (id, operation) = routes.camera.park(request);
                        shell.process_camera_effect(CameraEffect {
                            id: id.into_raw(),
                            operation,
                        });
                    }
                    // Original serialized FFI
                    effect => {
                        let serialized_effect = SerializedEffect::try_from(effect)
                            .expect("non-serialized effects are handled above");

                        let bytes = routes
                            .serialized
                            .serialize(serialized_effect)
                            .expect("serialized effect request should encode");

                        shell.process_serialized_effects(bytes);
                    }
                }
            });

            Self { router }
        }

        // The FFI below is fully controlled by the given core

        pub(crate) fn update(&self, event: Event) {
            self.router.update(event);
        }

        /// Resolve an effect sent over the serialized lane.
        pub(crate) fn resolve_serialized(&self, id: u32, data: &[u8]) {
            self.router
                .routes
                .serialized
                .resolve(BridgeEffectId(id), data)
                .expect("serialized resolve should work");
        }

        /// Specific effect FFI for opaque data types
        pub(crate) fn resolve_camera(&self, id: u64, output: OpaqueImageRef) {
            self.router
                .routes
                .camera
                .resolve(EffectId::from_raw(id), output)
                .expect("camera resolve should work");
        }

        pub(crate) fn view(&self) -> app::ViewModel {
            self.router.view()
        }
    }

    // Example core-side effect handler implementation
    mod file_store {
        use std::{sync::Weak, thread, time::Duration};

        use crossbeam_channel::{Receiver, Sender, unbounded};
        use crux_core::{Request, effects::ResolveSink};

        use crate::app::StoreFile;

        use super::app;

        pub(crate) struct FileStoreHandler {
            jobs_tx: Sender<Request<StoreFile>>,
        }

        impl FileStoreHandler {
            pub(crate) fn new<S>(sink: Weak<S>) -> Self
            where
                S: ResolveSink<StoreFile> + Send + Sync + 'static,
            {
                let (jobs_tx, jobs_rx) = unbounded();

                thread::spawn(move || worker(&jobs_rx, &sink));

                Self { jobs_tx }
            }

            pub(crate) fn process_file_store(&self, request: Request<StoreFile>) {
                self.jobs_tx
                    .send(request)
                    .expect("file store worker queue disconnected");
            }
        }

        fn worker<S>(jobs_rx: &Receiver<Request<StoreFile>>, sink: &Weak<S>)
        where
            S: ResolveSink<StoreFile> + Send + Sync + 'static,
        {
            while let Ok(mut request) = jobs_rx.recv() {
                // Ensure async behavior so this path models background work.
                thread::sleep(Duration::from_millis(1));

                let image_id = request.operation.image.0;
                let stored = app::StoredImage {
                    path: format!("/tmp/selfie-{image_id}.jpg"),
                };

                if let Some(sink) = sink.upgrade() {
                    sink.resolve_request(&mut request, stored)
                        .expect("background file store resolve should succeed");
                }
            }
        }
    }
}

/// Pretend shell for testing purposes, implements `CameraShell` like a real shell would via `uniffi` or `wasm_bindgen`
/// This is only a test stub. In real life the shell would process the effect, then call the core with a result,
/// in this case the tests do those steps
#[derive(Default)]
pub(crate) struct TestShell {
    pub(crate) serialized: Mutex<Vec<Vec<u8>>>,
    pub(crate) camera: Mutex<Vec<CameraEffect>>,
}

impl ffi::CameraShell for TestShell {
    fn process_serialized_effects(&self, bytes: Vec<u8>) {
        self.serialized
            .lock()
            .expect("fake shell serialized lock poisoned")
            .push(bytes);
    }

    fn process_camera_effect(&self, effect: CameraEffect) {
        self.camera
            .lock()
            .expect("fake shell camera lock poisoned")
            .push(effect);
    }
}

impl TestShell {
    /// Test-only helper: take serialized effects emitted from the core
    pub(crate) fn take_serialized_effects(&self) -> Vec<FfiRequest> {
        let payloads = self
            .serialized
            .lock()
            .expect("fake shell serialized lock poisoned")
            .drain(..)
            .collect::<Vec<_>>();

        payloads
            .into_iter()
            .flat_map(|bytes| {
                serde_json::from_slice::<Vec<FfiRequest>>(&bytes)
                    .expect("serialized shell payload should decode")
            })
            .collect()
    }

    /// Test only helper: take camera effects emitted from the core
    pub(crate) fn take_camera_effects(&self) -> Vec<CameraEffect> {
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
    let core = CameraFFI::new(shell.clone());

    core.update(Event::Trigger);

    assert_eq!(
        core.view(),
        ViewModel {
            countdown: None,
            save_status: SaveStatus::WaitingForPermission,
            saved_path: None,
        }
    );

    let serialized = shell.take_serialized_effects();
    assert!(has_render_effect(&serialized));
    let (permission_id, permission_request) = extract_single_permission_request(serialized);
    assert_eq!(permission_request, PermissionRequest::Camera);

    let camera_effects = shell.take_camera_effects();
    assert_eq!(camera_effects.len(), 0);

    let permission_response = PermissionResponse::Granted;
    let response_bytes =
        serde_json::to_vec(&permission_response).expect("permission response should serialize");
    core.resolve_serialized(permission_id, &response_bytes);

    let camera_effects = shell.take_camera_effects();
    assert_eq!(camera_effects.len(), 1);
    assert_eq!(camera_effects[0].operation, CaptureImageOp);

    let serialized = shell.take_serialized_effects();
    assert_only_render_effect(&serialized);

    assert_eq!(
        core.view(),
        ViewModel {
            countdown: None,
            save_status: SaveStatus::WaitingForCapture,
            saved_path: None,
        }
    );
}

#[test]
fn trigger_with_timer_takes_a_picture_after_countdown() {
    let shell = Arc::new(TestShell::default());
    let core = CameraFFI::new(shell.clone());

    core.update(Event::TriggerWithTimer);

    assert_eq!(
        core.view(),
        ViewModel {
            countdown: None,
            save_status: SaveStatus::WaitingForPermission,
            saved_path: None,
        }
    );

    let mut serialized = shell.take_serialized_effects();
    assert!(has_render_effect(&serialized));
    let (permission_id, permission_request) = extract_single_permission_request(serialized);
    assert_eq!(permission_request, PermissionRequest::Camera);

    let permission_response = PermissionResponse::Granted;
    let permission_bytes =
        serde_json::to_vec(&permission_response).expect("permission response should serialize");
    core.resolve_serialized(permission_id, &permission_bytes);

    assert_eq!(
        core.view(),
        ViewModel {
            countdown: Some(3),
            save_status: SaveStatus::WaitingForCapture,
            saved_path: None,
        }
    );

    serialized = shell.take_serialized_effects();
    assert!(has_render_effect(&serialized));

    // 3 -> 2
    let (request_id, request) = extract_single_time_request(serialized);
    let TimeRequest::NotifyAfter { id: timer_id, .. } = request else {
        panic!("expected NotifyAfter request")
    };
    let response = TimeResponse::DurationElapsed { id: timer_id };
    let response_bytes = serde_json::to_vec(&response).expect("time response should serialize");
    core.resolve_serialized(request_id, &response_bytes);

    assert_eq!(core.view().countdown, Some(2));
    serialized = shell.take_serialized_effects();
    assert!(has_render_effect(&serialized));

    // 2 -> 1
    let (request_id, request) = extract_single_time_request(serialized);
    let TimeRequest::NotifyAfter { id: timer_id, .. } = request else {
        panic!("expected NotifyAfter request")
    };
    let response = TimeResponse::DurationElapsed { id: timer_id };
    let response_bytes = serde_json::to_vec(&response).expect("time response should serialize");
    core.resolve_serialized(request_id, &response_bytes);

    assert_eq!(core.view().countdown, Some(1));
    serialized = shell.take_serialized_effects();
    assert!(has_render_effect(&serialized));

    // 1 -> camera capture
    let (request_id, request) = extract_single_time_request(serialized);
    let TimeRequest::NotifyAfter { id: timer_id, .. } = request else {
        panic!("expected NotifyAfter request")
    };
    let response = TimeResponse::DurationElapsed { id: timer_id };
    let response_bytes = serde_json::to_vec(&response).expect("time response should serialize");
    core.resolve_serialized(request_id, &response_bytes);

    assert_eq!(core.view().countdown, None);

    let camera_effects = shell.take_camera_effects();
    assert_eq!(camera_effects.len(), 1);
    assert_eq!(camera_effects[0].operation, CaptureImageOp);

    let serialized = shell.take_serialized_effects();
    assert_only_render_effect(&serialized);
}

#[test]
fn pictures_are_saved_to_file_system() {
    let shell = Arc::new(TestShell::default());
    let core = CameraFFI::new(shell.clone());

    core.update(Event::Trigger);

    let serialized = shell.take_serialized_effects();
    let (permission_id, permission_request) = extract_single_permission_request(serialized);
    assert_eq!(permission_request, PermissionRequest::Camera);

    let permission_response = PermissionResponse::Granted;
    let permission_bytes =
        serde_json::to_vec(&permission_response).expect("permission response should serialize");
    core.resolve_serialized(permission_id, &permission_bytes);

    let camera_effect = shell.take_camera_effects().remove(0);

    // Drain render from permission resolution; focus assertions on the save flow.
    let _ = shell.take_serialized_effects();

    core.resolve_camera(camera_effect.id, OpaqueImageRef(42));

    assert_eq!(
        core.view(),
        ViewModel {
            countdown: None,
            save_status: SaveStatus::Saving,
            saved_path: None,
        }
    );

    let serialized = shell.take_serialized_effects();
    assert_only_render_effect(&serialized);

    for _ in 0..50 {
        if core.view().save_status == SaveStatus::Saved {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    assert_eq!(
        core.view(),
        ViewModel {
            countdown: None,
            save_status: SaveStatus::Saved,
            saved_path: Some("/tmp/selfie-42.jpg".to_string()),
        }
    );

    let serialized = shell.take_serialized_effects();
    assert_only_render_effect(&serialized);
}

// helpers

fn has_render_effect(effects: &[FfiRequest]) -> bool {
    effects
        .iter()
        .any(|request| matches!(request.effect, FfiEffect::Render(_)))
}

fn assert_only_render_effect(effects: &[FfiRequest]) {
    assert_eq!(effects.len(), 1);
    assert!(matches!(effects[0].effect, FfiEffect::Render(_)));
}

pub(crate) fn extract_single_time_request(effects: Vec<FfiRequest>) -> (u32, TimeRequest) {
    let mut time_effects: Vec<_> = effects
        .into_iter()
        .filter_map(|effect| match effect.effect {
            FfiEffect::Time(request) => Some((effect.id.0, request)),
            FfiEffect::Render(_) | FfiEffect::Permission(_) => None,
        })
        .collect();

    assert_eq!(time_effects.len(), 1, "expected exactly one time request");

    time_effects.remove(0)
}

pub(crate) fn extract_single_permission_request(
    effects: Vec<FfiRequest>,
) -> (u32, PermissionRequest) {
    let mut permission_effects: Vec<_> = effects
        .into_iter()
        .filter_map(|effect| match effect.effect {
            FfiEffect::Permission(request) => Some((effect.id.0, request)),
            FfiEffect::Render(_) | FfiEffect::Time(_) => None,
        })
        .collect();

    assert_eq!(
        permission_effects.len(),
        1,
        "expected exactly one permission request"
    );

    permission_effects.remove(0)
}
