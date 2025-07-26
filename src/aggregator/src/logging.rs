use once_cell::sync::OnceCell;
use tracing::Level;
use tracing_subscriber::fmt;

static INIT: OnceCell<()> = OnceCell::new();

#[cfg(target_arch = "wasm32")]
struct IcWriter;
#[cfg(target_arch = "wasm32")]
impl std::io::Write for IcWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        ic_cdk::println!("{s}");
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn init() {
    INIT.get_or_init(|| {
        let level = option_env!("LOG_LEVEL").unwrap_or("info").to_lowercase();
        let lvl = match level.as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        };
        #[cfg(target_arch = "wasm32")]
        {
            let subscriber = fmt::Subscriber::builder()
                .with_max_level(lvl)
                .with_ansi(false)
                .with_writer(|| IcWriter)
                .without_time()
                .finish();
            let _ = tracing::subscriber::set_global_default(subscriber);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let subscriber = fmt::Subscriber::builder()
                .with_max_level(lvl)
                .with_target(false)
                .finish();
            let _ = tracing::subscriber::set_global_default(subscriber);
        }
    });
}
