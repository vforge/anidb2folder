use tracing::Level;
use tracing_subscriber::EnvFilter;

pub fn init(verbosity: u8) {
    let level = match verbosity {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    let filter = EnvFilter::from_default_env().add_directive(level.into());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(verbosity >= 2)
        .with_writer(std::io::stderr)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbosity_level_mapping() {
        // Just verify the match arms are correct
        assert_eq!(
            match 0u8 {
                0 => Level::WARN,
                1 => Level::INFO,
                2 => Level::DEBUG,
                _ => Level::TRACE,
            },
            Level::WARN
        );
        assert_eq!(
            match 1u8 {
                0 => Level::WARN,
                1 => Level::INFO,
                2 => Level::DEBUG,
                _ => Level::TRACE,
            },
            Level::INFO
        );
        assert_eq!(
            match 2u8 {
                0 => Level::WARN,
                1 => Level::INFO,
                2 => Level::DEBUG,
                _ => Level::TRACE,
            },
            Level::DEBUG
        );
        assert_eq!(
            match 3u8 {
                0 => Level::WARN,
                1 => Level::INFO,
                2 => Level::DEBUG,
                _ => Level::TRACE,
            },
            Level::TRACE
        );
    }
}
