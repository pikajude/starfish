use std::borrow::Cow;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use actix_web::{get, web, Responder};
use actix_web_lab::sse;
use futures_util::StreamExt;
use inotify::{EventMask, Inotify, WatchMask};
use log::info;
use serde::{Deserialize, Serialize};
use starfish::{BoxDynError, WorkerConfig};

#[derive(Deserialize)]
pub struct LenSpec {
  len: Option<usize>,
}

#[get("/build/{id}/tail")]
pub async fn get_build_tail(
  wc: web::Data<WorkerConfig>,
  id: web::Path<i32>,
  len: web::Query<LenSpec>,
) -> actix_web::Result<impl Responder> {
  let tail_len = len.len.unwrap_or(20);
  let log_path = wc.logfile(*id);

  let logfile = File::open(&log_path)?;

  let (sender, sse_stream) = sse::channel(10);

  let jh = actix_web::rt::spawn(async move {
    if let Err(e) = tail_the_file(sender, &log_path, logfile, tail_len).await {
      info!("client thread exited: {:?}", e);
    }
  });
  std::mem::forget(jh);

  Ok(sse_stream)
}

async fn tail_the_file(
  sender: sse::Sender,
  log_path: &Path,
  mut logfile: File,
  tail_len: usize,
) -> Result<(), BoxDynError> {
  let tailhead = tailme(&mut logfile, tail_len, b'\n')?;

  #[derive(Serialize)]
  #[serde(tag = "t", content = "c")]
  enum TailEvent<'s> {
    Text(Cow<'s, str>),
    Error(String),
    Reset,
  }

  macro_rules! yield_ {
    ($e:expr) => {
      sender
        .send(sse::Event::Data(sse::Data::new_json(&$e).unwrap()))
        .await?;
    };
  }

  yield_!(TailEvent::Text(String::from_utf8_lossy(&tailhead)));

  let mut notifier = Inotify::init()?;
  notifier.add_watch(log_path, WatchMask::CREATE | WatchMask::MODIFY)?;
  let buffer = vec![0u8; 1024];
  let mut notifs = notifier.event_stream(buffer)?;

  while let Some(ev) = notifs.next().await {
    let event_err: Result<(), BoxDynError> = try {
      let ev = ev?;
      if ev.mask.contains(EventMask::MODIFY) {
        let mut v = vec![];
        logfile.read_to_end(&mut v)?;
        if !v.is_empty() {
          yield_!(TailEvent::Text(String::from_utf8_lossy(&v)));
        }
      } else if ev.mask.contains(EventMask::CREATE) {
        yield_!(TailEvent::Reset);
        break;
      }
    };
    if let Err(e) = event_err {
      yield_!(TailEvent::Error(e.to_string()));
      break;
    }
  }

  Ok(())
}

// it may not look like much, but it's honest work (copied from GNU tail)
fn tailme(fd: &mut File, mut n_lines: usize, sep: u8) -> std::io::Result<Vec<u8>> {
  const BUFSIZE: usize = 1024;

  if n_lines == 0 {
    return Ok(vec![]);
  }

  let end_pos = fd.seek(SeekFrom::End(0))?;
  let start_pos = 0;

  let mut buffer = [0u8; BUFSIZE];
  let mut pos = end_pos;

  let mut bytes_read = ((pos - start_pos) as usize) % BUFSIZE;
  if bytes_read == 0 {
    bytes_read = BUFSIZE;
  }

  pos -= bytes_read as u64;
  fd.seek(SeekFrom::Start(pos as _))?;
  bytes_read = fd.read(&mut buffer)?;
  if bytes_read > 0 && buffer[bytes_read - 1] != sep {
    n_lines -= 1;
  }

  loop {
    let mut n = bytes_read;
    while n > 0 {
      n = match buffer[..n].iter().rposition(|c| *c == sep) {
        Some(c) => c,
        None => break,
      };
      if n_lines == 0 {
        let mut v = buffer[n + 1..bytes_read].to_vec();
        BufReader::new(fd)
          .take(end_pos - (pos + bytes_read as u64))
          .read_to_end(&mut v)?;
        return Ok(v);
      }
      n_lines -= 1;
    }

    if pos == start_pos {
      fd.seek(SeekFrom::Start(start_pos))?;
      let mut v = vec![];
      BufReader::new(fd).take(end_pos).read_to_end(&mut v)?;
      return Ok(v);
    }

    pos -= BUFSIZE as u64;
    fd.seek(SeekFrom::Start(pos))?;

    bytes_read = fd.read(&mut buffer)?;

    if bytes_read == 0 {
      break;
    }
  }

  Ok(vec![])
}
