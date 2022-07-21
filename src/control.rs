
use futures::AsyncReadExt;
use anyhow::{Result, Error};
use yamux::{Stream};
use crate::{
    crypto::FrpCoder,
    service::Service,
    msg::{ReqWorkConn, NewWorkConn,  msg_header_decode, MsgHeader, MSG_HEADER_SIZE},
};

#[derive(Debug, Clone)]
pub struct Control {
    coder:      FrpCoder,
    service:    Service,
}

impl Control {

    pub fn new(service: Service, iv: [u8; 16]) -> Self {
        
        let mut coder = FrpCoder::new(service.cfg.auth_token().to_string(), iv);

        Self {
            coder,
            service,
        }
    }

    pub async fn run(&mut self, main_stream: &mut Stream) -> Result<()> {
        
        loop {
            let mut buf = [0; 4096];
            let n = main_stream.read(&mut buf).await?;
            assert_eq!((n < 4096), true);
            println!("read msg length {}", n);
            let mut plain_msg = buf[0..n].to_vec();
            self.coder.clone().decrypt(&mut plain_msg);
            let hdr: [u8; MSG_HEADER_SIZE] = plain_msg[0..MSG_HEADER_SIZE].try_into().expect("slice with incorrect length");
            let header: MsgHeader = msg_header_decode(&hdr);
            assert_eq!(header.len as usize, n - MSG_HEADER_SIZE);
            println!("header {:?}", header);
            self.handle_msg(&header, &plain_msg[MSG_HEADER_SIZE..n]).await?;
        }
    }

    pub async fn handle_msg(&mut self, header: &MsgHeader, msg: &[u8]) -> Result<()> {
        match header.msg_type {
           TypeReqWorkConn => self.handle_req_work_conn().await,
           _ => Err(anyhow::anyhow!("unsupported type {:?}", header)),
        }
    }

    pub async fn handle_req_work_conn(&mut self) -> Result<()> {
        let work_conn = NewWorkConn::new(self.service.run_id.clone(), &self.service.cfg); 
        let mut work_stream = self.service.main_ctl.open_stream().await.unwrap();

        Ok(())
    }
}
