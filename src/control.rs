
use anyhow::{Result, Error};
use yamux::{Stream};
use futures_util::io::{AsyncReadExt, AsyncWriteExt};
use std::{
    collections::HashMap,
    str
};
use tokio::{net::TcpStream, time::timeout};

use crate::{
    crypto::FrpCoder,
    service::Service,
    config:: {
        ClientTcpConfig,
        ClientWebConfig
    },
    msg::{
        ReqWorkConn, 
        NewWorkConn,  
        NewProxy,
        msg_header_decode, 
        msg_header_encode,
        MsgHeader, 
        MSG_HEADER_SIZE,
        TypeNewProxyResp,
        TypeNewWorkConn,
        StartWorkConn,
        TypeReqWorkConn
    },
};

#[derive(Debug, Clone)]
pub struct Control {
    coder:      FrpCoder,
    service:    Service,
    send_proxy: bool,
}

impl Control {

    pub fn new(service: Service, iv: [u8; 16]) -> Self {
        
        let mut coder = FrpCoder::new(service.cfg.auth_token(), iv);

        Self {
            coder,
            service,
            send_proxy: false,
        }
    }

    pub async fn run(&mut self, main_stream: &mut Stream) -> Result<()> {

        loop {
            let mut buf = [0; 4096];
            let n = main_stream.read(&mut buf).await?;
            let mut plain_msg = buf[0..n].to_vec();
            self.coder.decrypt(&mut plain_msg);
            let hdr: [u8; MSG_HEADER_SIZE] = 
                plain_msg[0..MSG_HEADER_SIZE].try_into().expect("slice with incorrect length");
            let header: MsgHeader = msg_header_decode(&hdr);
            assert_eq!(header.len as usize, n - MSG_HEADER_SIZE);
            self.handle_msg(&header, &plain_msg[MSG_HEADER_SIZE..n]).await;
            
            self.send_proxy_conf(main_stream).await;
        }
    }

    pub async fn handle_msg(&mut self, header: &MsgHeader, msg: &[u8]) -> Result<()> {
        println!("msg_type {:?}", header.msg_type);
        match header.msg_type {
           TypeNewProxyResp => self.handle_new_proxy_resp(msg).await,
           TypeReqWorkConn => self.handle_req_work_conn().await,
           _ => Err(anyhow::anyhow!("unsupported type {:?}", header)),
        }
    }

    async fn handle_req_work_conn(&mut self) -> Result<()> {
        let work_conn = NewWorkConn::new(self.service.run_id.clone(), &self.service.cfg); 
        let mut work_stream = self.service.main_ctl.open_stream().await.unwrap();
        let frame = work_conn.to_string().into_bytes();
        let hdr = MsgHeader::new(TypeNewWorkConn, frame.len() as u64);
        work_stream.write_all(&msg_header_encode(&hdr).to_vec()).await;
        work_stream.write_all(&frame).await;
        
        let conf = self.service.get_conf().clone();
        tokio::spawn(async move {
            println!("waiting for NewWorkConn command");
            let mut msg_hdr = [0; MSG_HEADER_SIZE];
            work_stream.read_exact(&mut msg_hdr).await;
            let header: MsgHeader = msg_header_decode(&msg_hdr.try_into().unwrap());
            let mut msg = vec![0; header.len as usize];
            work_stream.read_exact(&mut msg).await;
            let resp = String::from_utf8_lossy(&msg);
            let start_work_conn: StartWorkConn = serde_json::from_str(&resp).unwrap();
            println!("start_work_conn {:?}", start_work_conn);

            let proxy = conf.get_proxy(&start_work_conn.proxy_name).unwrap();
            let mut local_stream = 
                TcpStream::connect(format!("{}:{}", proxy.server_addr, proxy.server_port)).await;
        });

        Ok(())
    }
    
    async fn handle_new_proxy_resp(&mut self, msg: &[u8]) -> Result<()> {
        let res = str::from_utf8(msg).unwrap();
        println!("new proxy response {}", res);
        
        Ok(())
    }

    async fn send_proxy_conf(&mut self, main_stream: &mut Stream) -> Result<()> {
        if self.send_proxy {
            println!("already send proxy conf");
            return Ok(());
        }

        let iv = self.coder.iv();
        main_stream.write_all(iv).await?;

        let mut cfg = self.service.get_conf().clone();
        self.send_tcp_proxy_conf(main_stream, &cfg.tcp_configs).await?;
        self.send_web_proxy_conf(main_stream, &cfg.web_configs).await?;
        
        self.send_proxy = true;

        Ok(())
    }

    async fn send_tcp_proxy_conf(
        &mut self, 
        main_stream: &mut Stream, 
        configs:     &HashMap<String, ClientTcpConfig>
    ) -> Result<()> {

        for (proxy_name, tcp_config) in configs {
            let mut new_proxy = NewProxy::new(&proxy_name, &tcp_config.service_type);
            new_proxy.set_remote_port(tcp_config.remote_port);
            new_proxy.send_msg(main_stream, &mut self.coder).await?;
        }

        Ok(())
    }

    async fn send_web_proxy_conf(
        &mut self, 
        main_stream: &mut Stream, 
        configs:     &HashMap<String, ClientWebConfig>
    ) -> Result<()> {
        
        for (proxy_name, web_config) in configs {
            let mut new_proxy = NewProxy::new(&proxy_name, &web_config.service_type);
            if !web_config.custom_domains.is_none() {
                let mut domains = Vec::new();
                let custom_domain = web_config.custom_domains.as_ref().unwrap();
                domains.push(custom_domain.to_string());
                new_proxy.set_custom_domains(&domains);
            }
            if !web_config.subdomain.is_none() {
                new_proxy.set_subdomain(web_config.subdomain.as_ref().unwrap());
            }
            
            new_proxy.send_msg(main_stream, &mut self.coder).await?;
        }

        Ok(())
    }
}
