use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use log::info;

use pbr::ProgressBar;
use progress_streams::ProgressReader;

pub fn download_file(url: &str) -> Result<Vec<u8>, &'static str> {
    info!("downloading pdb from {}", url);
    let resp = ureq::get(url).call();
    if !resp.ok() {
        return Err("unable to download pdb");
    }

    assert!(resp.has("Content-Length"));
    let len = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap();

    let mut reader = resp.into_reader();
    let buffer = read_to_end(&mut reader, len)?;

    assert_eq!(buffer.len(), len);
    Ok(buffer)
}

fn read_to_end<T: Read>(reader: &mut T, len: usize) -> Result<Vec<u8>, &'static str> {
    let mut buffer = vec![];

    let total = Arc::new(AtomicUsize::new(0));
    let mut reader = ProgressReader::new(reader, |progress: usize| {
        total.fetch_add(progress, Ordering::SeqCst);
    });
    let mut pb = ProgressBar::new(len as u64);

    let finished = Arc::new(AtomicBool::new(false));
    let thread = {
        let finished_thread = finished.clone();
        let total_thread = total.clone();

        std::thread::spawn(move || {
            while !finished_thread.load(Ordering::Relaxed) {
                pb.set(total_thread.load(Ordering::SeqCst) as u64);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            pb.finish();
        })
    };

    reader
        .read_to_end(&mut buffer)
        .map_err(|_| "unable to read from http request")?;
    finished.store(true, Ordering::Relaxed);
    thread.join().unwrap();

    Ok(buffer)
}
