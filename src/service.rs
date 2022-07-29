
use anyhow::{Context, Result};
use tokio::{net::TcpSocket, runtime::Runtime, task};
use tokio_util::compat::TokioAsyncReadCompatExt;
use yamux::{Config as YamuxConfig, Connection, Control, Stream, Mode, WindowUpdateMode};
use std::process;

use crate::config::Config;
use crate::msg::{Login, LoginResp};

#[derive(Debug)]
pub struct Service {
    main_ctl:       Control,
    main_stream:    Stream,
    run_id:         String,
    cfg:            Config,
}

impl Service {
    
    pub async fn new(cfg: Config) -> Result<Self> {
        let conn = {
            let mut yamux_cfg = YamuxConfig::default();
            yamux_cfg.set_split_send_size(crate::PAYLOAD_SIZE);
            yamux_cfg.set_window_update_mode(WindowUpdateMode::OnRead);
        
            let address = format!("{}:{}", cfg.server_addr(), cfg.server_port()).parse().unwrap();
            let socket = TcpSocket::new_v4().expect("new_v4");
            let stream = socket.connect(address).await.expect("connect").compat();
            Connection::new(stream, yamux_cfg, Mode::Client)
        };
        let mut ctrl = conn.control();
        let mut stream = ctrl.open_stream().await?;

        Ok(Self {
            main_ctl: ctrl,
            main_stream: stream,
            run_id: "".to_string(),
            cfg,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let login = Login::new(&self.cfg);
        let login_resp = login.send_msg(&mut self.main_stream).await.unwrap();
        if login_resp.error().is_empty() {
            println!("app exit {}", login_resp.error());
            process::exit(1);
        }
        assert_eq!(login_resp.run_id().is_empty(), false);
        self.run_id = login_resp.run_id();
        
        Ok(())
    }
}

