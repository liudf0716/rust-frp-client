use std::process::ExitCode;

use clap::Command;
use rust_frp_client::frpc;

fn main() -> ExitCode {
    let mut app = Command::new("frpc")
        .version(frp_rust::VERSION)
        .author("Dengfeng Liu <liu_df@qq.com>")
        .about("frpc is the client of frp");
    app = frpc::define_command_line_options(app);

    let matches = app.get_matches();
    frpc::main(&matches)
}
