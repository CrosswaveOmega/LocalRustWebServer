use tracing::Subscriber;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    filter::LevelFilter,
};
use tracing_subscriber::fmt::format::{Writer, FormatFields, FormatEvent};
use tracing_subscriber::registry::LookupSpan;
use tracing::Event;
use tracing_subscriber::fmt::{time::FormatTime};
use tracing::{error, info, warn};
use tracing_subscriber::Layer;
use tracing_subscriber::fmt::time::UtcTime;


use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex, Arc},
    cmp::Reverse,
    time::{SystemTime, Duration},
};

pub struct ManagedLogWriter {
    inner: tracing_appender::rolling::RollingFileAppender,
    last_cleanup: Mutex<SystemTime>,
    dir: PathBuf,
    prefix: String,
    keep: usize,
}

impl ManagedLogWriter {
    pub fn new(appender: tracing_appender::rolling::RollingFileAppender, dir: impl Into<PathBuf>, prefix: &str, keep: usize) -> Self {
        Self {
            inner: appender,
            last_cleanup: Mutex::new(SystemTime::UNIX_EPOCH),
            dir: dir.into(),
            prefix: prefix.to_string(),
            keep,
        }
    }

    fn maybe_cleanup(&self) {
        let mut last = self.last_cleanup.lock().unwrap();
        let now = SystemTime::now();

        // Avoid running cleanup too often (e.g. only once every 10 minutes)
        if now.duration_since(*last).unwrap_or(Duration::ZERO) < Duration::from_secs(600) {
            return;
        }

        *last = now;

        // Run cleanup
        if let Ok(entries) = fs::read_dir(&self.dir) {
            let mut files: Vec<_> = entries
                .filter_map(Result::ok)
                .filter(|e| {
                    e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                        && e.file_name().to_string_lossy().starts_with(&self.prefix)
                })
                .collect();

            files.sort_by_key(|e| {
                e.metadata()
                    .and_then(|m| m.modified())
                    .map(Reverse)
                    .unwrap_or(Reverse(SystemTime::UNIX_EPOCH))
            });

            for entry in files.iter().skip(self.keep) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

impl Write for ManagedLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.maybe_cleanup();
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}


pub struct CustomFormatter<T> {
    pub timer: T,
}

impl<S, N, T> FormatEvent<S, N> for CustomFormatter<T>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
    T: FormatTime,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let meta = event.metadata();

        write!(writer, "[LINE] [")?;
        self.timer.format_time(&mut writer)?; // <--- Use stored timer
        write!(
            writer,
            "] [{:<8}] {}: ",
            meta.level().as_str(),
            meta.target()
        )?;

        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}




pub fn init_logging() {
    // Create a daily rotating file appender in a relative path
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "./logs", "my_app.log");

    
    let managed_writer = ManagedLogWriter::new(file_appender, "./logs", "applog", 5);
    // Create a non-blocking writer and retain the guard to prevent early drop
    let (non_blocking, guard) = tracing_appender::non_blocking(managed_writer);

    // You must store this guard to keep the background logging thread alive
    // If dropped, logs will stop being written!
    Box::leak(Box::new(guard)); // <- safest simple method

    let custom_format = CustomFormatter { timer: UtcTime::rfc_3339() };
    tracing_subscriber::registry()
        .with(
            fmt::layer()
            .with_writer(non_blocking)
            .with_target(true) // includes module path
            .with_ansi(false)  // disable colors for file
            .event_format(custom_format)
            .with_filter(LevelFilter::INFO),
        )
        .init();

    // Log test messages
    info!("This is an info message.");
    warn!("This is a warning.");
    error!("This is an error.");
}
