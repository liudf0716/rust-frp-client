use anyhow::{Context, Result};
use futures::{channel::mpsc, prelude::*};
use std::process;
use tokio::{net::TcpSocket, runtime::Runtime, task};
use tokio_util::compat::TokioAsyncReadCompatExt;
use yamux::{Config as YamuxConfig, Connection, Control, Mode, Stream, WindowUpdateMode};

use crate::{
    config::Config,
    control::Control as FrpControl,
    msg::{Login, LoginResp},
};

#[derive(Debug, Clone)]
pub struct Service {
    pub main_ctl: Control,
    pub run_id: String,
    pub cfg: Config,
}

impl Service {
    pub async fn new(cfg: Config) -> Result<Self> {
        let conn = {
            let mut yamux_cfg = YamuxConfig::default();
            yamux_cfg.set_split_send_size(crate::PAYLOAD_SIZE);
            yamux_cfg.set_window_update_mode(WindowUpdateMode::OnRead);

            let address = format!("{}:{}", cfg.server_addr(), cfg.server_port())
                .parse()
                .unwrap();
            let socket = TcpSocket::new_v4().expect("new_v4");
            let stream = socket.connect(address).await.expect("connect").compat();
            Connection::new(stream, yamux_cfg, Mode::Client)
        };
        let mut ctrl = conn.control();
        task::spawn(yamux::into_stream(conn).for_each(|_| future::ready(())));

        Ok(Self {
            main_ctl: ctrl,
            run_id: "".to_string(),
            cfg,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut main_stream = self.main_ctl.open_stream().await.unwrap();
        let login = Login::new(&self.cfg);
        let login_resp = login.send_msg(&mut main_stream).await.unwrap();
        println!("login_resp {:?}", login_resp);
        if !login_resp.error().is_empty() {
            println!("app exit {}", login_resp.error());
            process::exit(1);
        }
        assert_eq!(login_resp.run_id().is_empty(), false);
        self.run_id = login_resp.run_id().to_string();

        // read iv[16]
        let mut iv = [0; 16];
        main_stream.read_exact(&mut iv).await?;
        println!("iv {:?}", iv);

        let mut frp_ctl = FrpControl::new(self.clone(), iv);
        frp_ctl.run(&mut main_stream).await?;

        Ok(())
    }

    pub fn get_conf(&self) -> &Config {
        &self.cfg
    }
}
