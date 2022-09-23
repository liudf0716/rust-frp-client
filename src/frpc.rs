use anyhow::Result;
use clap::{Arg, ArgGroup, ArgMatches, Command, ErrorKind as ClapErrorKind};
use log::{info, trace};
use std::process::ExitCode;

use crate::config::Config;
use crate::service::Service;

pub fn define_command_line_options(mut app: Command<'_>) -> Command<'_> {
    app = app.arg(
        Arg::new("config")
            .short('c')
            .long("config")
            .required(true)
            .takes_value(true)
            .help("frpc configuration file"),
    );

    app
}

#[tokio::main]
async fn start_service(config: Config) -> Result<()> {
    let mut service = Service::new(config).await?;
    service.run().await?;

    Ok(())
}

pub fn main(matches: &ArgMatches) -> ExitCode {
    let config_file = matches.value_of("config").unwrap();
    let mut client_config = Config::new();
    client_config.load_config(config_file);

    start_service(client_config);

    ExitCode::SUCCESS
}
