use x11rb::protocol::xproto::Window;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum WorkerKind {
    Monitor,
    Ocr,
    Activation,
}

pub(crate) enum WorkerEvent {
    Windows {
        windows: Vec<Window>,
        focused_window: Option<Window>,
    },
    Error {
        worker: WorkerKind,
        message: String,
    },
    Recovered(WorkerKind),
}
