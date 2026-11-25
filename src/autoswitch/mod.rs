use anyhow::{Context, Result};
use ocrs::{OcrEngine, OcrEngineParams};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::time::Duration;
use x11rb::errors::ReplyError;
use x11rb::rust_connection::RustConnection;

use crate::config::OcrRegion;
use crate::runtime::{WorkerEvent, WorkerKind};
use crate::window;

mod capture;
mod models;
mod preprocess;
mod ximage;

const OCR_INTERVAL: Duration = Duration::from_millis(250);

pub(crate) fn auto_switch(
    conn: &RustConnection,
    stop_rx: Receiver<()>,
    settings_rx: Receiver<Option<OcrRegion>>,
    events_tx: Sender<WorkerEvent>,
) -> Result<()> {
    let mut engine = None;
    let mut region = None;
    let mut dofus_windows = window::DofusWindowTracker::new();
    let mut last_ocr_error = None;

    loop {
        if matches!(stop_rx.try_recv(), Ok(()) | Err(TryRecvError::Disconnected)) {
            return Ok(());
        }
        while let Ok(next_region) = settings_rx.try_recv() {
            region = next_region;
        }

        let Some(region) = region else {
            report_error_recovered(&mut last_ocr_error, &events_tx);
            if wait_for_stop(&stop_rx, Duration::from_millis(100)) {
                return Ok(());
            }
            continue;
        };
        if engine.is_none() {
            match init_engine() {
                Ok(ready) => {
                    engine = Some(ready);
                    report_error_recovered(&mut last_ocr_error, &events_tx);
                }
                Err(error) => {
                    report_worker_error(
                        &mut last_ocr_error,
                        format!("Initialisation OCR impossible : {error:#}"),
                        &events_tx,
                    )?;
                    if wait_for_stop(&stop_rx, Duration::from_secs(2)) {
                        return Ok(());
                    }
                    continue;
                }
            }
        }

        match dofus_windows.refresh(conn) {
            Ok(Some(active_window)) => {
                match capture::process_capture(
                    conn,
                    active_window,
                    region,
                    engine.as_ref().unwrap(),
                ) {
                    Ok(detected) => {
                        report_error_recovered(&mut last_ocr_error, &events_tx);
                        let _ = events_tx.send(WorkerEvent::OcrText(detected.unwrap_or_default()));
                    }
                    Err(error) => {
                        if is_window_closed_error(&error) {
                            report_error_recovered(&mut last_ocr_error, &events_tx);
                            let _ = events_tx.send(WorkerEvent::OcrText(String::new()));
                        } else {
                            report_worker_error(
                                &mut last_ocr_error,
                                format!("Lecture OCR impossible : {error:#}"),
                                &events_tx,
                            )?;
                        }
                    }
                }
            }
            Ok(None) => {
                report_error_recovered(&mut last_ocr_error, &events_tx);
                let _ = events_tx.send(WorkerEvent::OcrText(String::new()));
            }
            Err(error) => report_worker_error(
                &mut last_ocr_error,
                format!("Suivi des fenêtres OCR impossible : {error:#}"),
                &events_tx,
            )?,
        }

        if wait_for_stop(&stop_rx, OCR_INTERVAL) {
            return Ok(());
        }
    }
}

fn wait_for_stop(stop_rx: &Receiver<()>, timeout: Duration) -> bool {
    !matches!(
        stop_rx.recv_timeout(timeout),
        Err(RecvTimeoutError::Timeout)
    )
}

fn report_worker_error(
    last_error: &mut Option<String>,
    message: String,
    events_tx: &Sender<WorkerEvent>,
) -> Result<()> {
    if last_error.as_deref() != Some(message.as_str()) {
        events_tx
            .send(WorkerEvent::Error {
                worker: WorkerKind::Ocr,
                message: message.clone(),
            })
            .map_err(|_| anyhow::anyhow!("GUI worker event channel is closed"))?;
        *last_error = Some(message);
    }
    Ok(())
}

fn report_error_recovered(last_error: &mut Option<String>, events_tx: &Sender<WorkerEvent>) {
    if last_error.take().is_some() {
        let _ = events_tx.send(WorkerEvent::Recovered(WorkerKind::Ocr));
    }
}

fn is_window_closed_error(err: &anyhow::Error) -> bool {
    matches!(
        err.downcast_ref::<ReplyError>(),
        Some(ReplyError::X11Error(x11_err)) if x11_err.error_code == 9 || x11_err.error_code == 3
    )
}

fn init_engine() -> Result<OcrEngine> {
    let (detection_model, recognition_model) = models::load_pair()?;

    OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })
    .context("Initialisation du moteur OCR impossible")
}
