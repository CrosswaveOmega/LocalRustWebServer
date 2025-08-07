use tracing::Event;
use tracing::Subscriber;
use tracing::{error, info, warn};
use tracing_subscriber::Layer;
use tracing_subscriber::fmt::format::{FormatEvent, FormatFields, Writer};
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use std::io::{Seek, SeekFrom};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    time::SystemTime,
};

const MAX_LOG_SIZE: u64 = 8 * 1024 * 1024; // 8 MB
const MAX_LOG_FILES: usize = 5;

/// A file appender with the ability to rotate log files should they
/// exceed a maximum size.
///
/// `SizeRotatingWriter` implements the [`std:io::Write` trait][write] and will
/// block on write operations.
///
/// `SizeRotatingWriter` does not implement the [`MakeWriter`]
/// trait yet from `tracing-subscriber`, so it may also be used
/// directly, without [`NonBlocking`].
///
/// [write]: std::io::Write
///
pub struct SizeRotatingWriter {
    base_path: PathBuf,
    current_file: fs::File,
    current_size: u64,
    prefix: String,
}

impl SizeRotatingWriter {
    pub fn new(log_dir: impl Into<PathBuf>, prefix: &str) -> io::Result<Self> {
        let dir = log_dir.into();
        fs::create_dir_all(&dir)?;

        let path = dir.join(format!("{}.log", prefix));
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        let size = file.seek(SeekFrom::End(0))?;

        Ok(Self {
            base_path: dir,
            current_file: file,
            current_size: size,
            prefix: prefix.to_string(),
        })
    }

    fn rotate(&mut self) -> io::Result<()> {
        // Close current file and rename with timestamp or number
        let mut rotated_files: Vec<_> = fs::read_dir(&self.base_path)?
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with(&self.prefix))
            .collect();

        rotated_files.sort_by_key(|e| {
            e.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        });
        while rotated_files.len() >= MAX_LOG_FILES {
            let oldest = rotated_files.remove(0);
            let _ = fs::remove_file(oldest.path());
        }

        let new_name = self.base_path.join(format!("{}.log", self.prefix));
        let current_log = self.base_path.join(format!("{}.log", self.prefix));

        fs::rename(&current_log, &new_name)?;

        // Open new file
        self.current_file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&current_log)?;

        self.current_size = 0;

        Ok(())
    }
}

impl Write for SizeRotatingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.current_size + buf.len() as u64 > MAX_LOG_SIZE {
            self.rotate()?;
        }

        let bytes_written = self.current_file.write(buf)?;
        self.current_size += bytes_written as u64;
        Ok(bytes_written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.current_file.flush()
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

/// Setup the logging system
pub fn init_logging() {
    let writer = SizeRotatingWriter::new("./logs", "my_app").expect("Failed to init log writer");
    let (non_blocking, guard) = tracing_appender::non_blocking(writer);

    // Store the guard to keep the background logging thread alive
    // If dropped, logs will stop being written!
    Box::leak(Box::new(guard)); // <- safest simple method

    let custom_format = CustomFormatter {
        timer: UtcTime::rfc_3339(),
    };
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_target(true) // includes module path
                .with_ansi(false) // disable colors for file
                .event_format(custom_format)
                .with_filter(LevelFilter::INFO),
        )
        .init();

    // Log test messages
    info!("This is an info message.");
    warn!("This is a warning.");
    error!("This is an error.");
}
