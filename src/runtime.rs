use x11rb::protocol::xproto::Window;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum WorkerKind {
    Monitor,
    Ocr,
}

pub(crate) enum WorkerEvent {
    Windows(Vec<Window>),
    Error { worker: WorkerKind, message: String },
    Recovered(WorkerKind),
}
