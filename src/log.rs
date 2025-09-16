use std::{env, fs::OpenOptions, str::FromStr as _};

use reopen::Reopen;
use simplelog::{CombinedLogger, ConfigBuilder, LevelFilter, SharedLogger, TermLogger, WriteLogger};

const LOG_SINKS: [&str; 2] = ["app", "access"];

pub fn logging_bootstrap(app_name: &str) {
    let mut sinks = Vec::<Box<dyn SharedLogger>>::with_capacity(LOG_SINKS.len());
    
    let config = ConfigBuilder::new()
        .set_target_level(LevelFilter::Error)
        .set_time_format_rfc3339()
        .set_level_color(simplelog::Level::Error, Some(simplelog::Color::Red))
        .set_level_color(simplelog::Level::Info, Some(simplelog::Color::Green)).to_owned();
    
    for sink in LOG_SINKS {
        let env_var_name = format!("{}_LEVEL", sink.to_uppercase());
        let level_env_var_name = format!("{}_LEVEL", &env_var_name);
        let log_level = env::var(&level_env_var_name).unwrap_or(String::from("info"));
        let log_level = LevelFilter::from_str(&log_level).expect(&format!("Invalid log level for {}: {}", &level_env_var_name, log_level));
        let config = config.clone().add_filter_allow(format!("{}::{}", app_name, sink)).build();
        let env_var = env::var(&env_var_name).unwrap_or(String::from("stderr"));
        sinks.push(match env_var.as_ref() {
            "stderr" => TermLogger::new(log_level, config, simplelog::TerminalMode::Stderr, simplelog::ColorChoice::Auto),
            file => {
                let file_name = String::from(file);
                let fd = Reopen::new(Box::new(move ||open_log_file(file_name.to_owned()))).expect(&format!("Unable to open log file for {}: {}", &env_var_name, file));
                // If system is unix, 
                #[cfg(unix)]
                actix_web::rt::spawn(async || {
                    let mut sig = signal(SignalKind::hangup())?;
                    loop {
                        sig.recv().await;
                        fd.handle().reopen();
                    }
                });
                WriteLogger::new(log_level, config, fd)
            }
        });
    };
    CombinedLogger::init(sinks).expect("Could not load log sinks!");
}

fn open_log_file(log_file: String) -> std::io::Result<std::fs::File> {
    OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(log_file)
}