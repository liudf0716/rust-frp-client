
use serde::{Serialize, Deserialize};
use std::{
    env::consts,
    mem::size_of,
    collections::HashMap,
};
use anyhow::Result;
use md5;
use chrono::Utc;
use yamux::{Stream}; 
use futures_util::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    config::Config,
    crypto::FrpCoder,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Login {
    version:    String,
    hostname:   String,
    os:         String,
    arch:       String,
    user:       String,
    privilege_key:  String,
    timestamp:  i64,
    metas:      HashMap<String, String>,
    pool_count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoginResp {
    version:    String,
    run_id:     Option<String>,
    error:      Option<String>,
}

impl Login {
    pub fn new(cfg: &Config) -> Self {
        let timestamp = Utc::now().timestamp();
        let privilege_key = get_privilege_key(timestamp, cfg.auth_token());
        let metas = HashMap::new();

        Self {
            version: crate::FRP_VERSION.to_string(),
            hostname: "".to_string(),
            os: consts::OS.to_string(),
            arch: consts::ARCH.to_string(),
            user: "rust-frp-client".to_string(),
            privilege_key,
            timestamp,
            metas,
            pool_count: 1,
        }
    }

    pub async fn send_msg(&self, main_stream: &mut Stream) -> Result<LoginResp> {
        let frame = self.to_string().into_bytes();
        let hdr = MsgHeader::new(TypeLogin, frame.len() as u64); 
        main_stream.write_all(&msg_header_encode(&hdr).to_vec()).await?;
        main_stream.write_all(&frame).await?;

        let mut msg_hdr = [0; MSG_HEADER_SIZE];
        main_stream.read_exact(&mut msg_hdr).await?;
        let header: MsgHeader = msg_header_decode(&msg_hdr.try_into().unwrap());
        let mut msg = vec![0; header.len as usize]; 
        main_stream.read_exact(&mut msg).await?;
        let resp = String::from_utf8_lossy(&msg);

        Ok(serde_json::from_str(&resp)?)
    }

    fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

}

impl LoginResp {
    pub fn run_id(& self) -> & str {
        match self.run_id {
            None => "",
            _ => &self.run_id.as_ref().unwrap(),
        }
    }

    pub fn error(&self) -> &str {
        match self.error {
            None => "",
            _ => &self.error.as_ref().unwrap(),
        }
    }
}

fn get_privilege_key(timestamp: i64, auth_token: &str) -> String {
    let seed = format!("{}{}", auth_token, timestamp);
    let digest = md5::compute(seed);

    format!("{:x}", digest)
}

pub struct ReqWorkConn;

impl ReqWorkConn {

    pub async fn handle_req_work_conn(main_stream: &mut Stream, decoder: &mut FrpCoder) -> Result<()> {
        let mut buf = [0; 128];
        let n = main_stream.read(&mut buf).await?;
        assert_eq!((n < 128), true);
        println!("ReqWorkConn read {}", n);
        let mut dbuf = buf[0..n].to_vec();
        decoder.decrypt(&mut dbuf);
        println!("dbuf {:?}", dbuf);

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewWorkConn {
    run_id:         String,
    privilege_key:  String,
    timestamp:      i64,
}

impl NewWorkConn {
    
    pub fn new(run_id: String, cfg: &Config) -> Self {
        let timestamp = Utc::now().timestamp();
        let privilege_key = get_privilege_key(timestamp, cfg.auth_token());

        Self {
            run_id,
            privilege_key,
            timestamp,
        }
    }
    
    fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewProxy {
    proxy_name:     String,
    proxy_type:     String,
     #[serde(skip_serializing_if = "Option::is_none")]
    remote_port:    Option<u16>,
     #[serde(skip_serializing_if = "Option::is_none")]
    custom_domains: Option<Vec<String>>,
     #[serde(skip_serializing_if = "Option::is_none")]
    subdomain:      Option<String>,
}

impl NewProxy {
    
    pub fn new(proxy_name: &str, proxy_type: &str) -> Self {

        Self {
            proxy_name: proxy_name.to_string(),
            proxy_type: proxy_type.to_string(),
            remote_port:    None,
            custom_domains: None,
            subdomain:      None,
        }
    }

    pub fn set_remote_port(&mut self, remote_port: u16) {
        self.remote_port = Some(remote_port)
    }
    
    pub fn set_custom_domains(&mut self, custom_domains: &Vec<String>) {
        self.custom_domains = Some(custom_domains.clone())
    }

    pub fn set_subdomain(&mut self, subdomain: &str) {
        self.subdomain = Some(subdomain.to_string())
    }
    
    pub async fn send_msg(&self, main_stream: &mut Stream, encoder: &mut FrpCoder) -> Result<()> {
        let frame = self.to_string().into_bytes();
        let cap = frame.len() + MSG_HEADER_SIZE;
        let mut data: Vec<u8> = vec![0; cap];
        data[0] = TypeNewProxy.0;
        data[1..MSG_HEADER_SIZE].copy_from_slice(&frame.len().to_be_bytes());
        data[MSG_HEADER_SIZE..].copy_from_slice(&frame);

        encoder.encypt(&mut data);
        main_stream.write_all(&data).await?;

        Ok(())
    }

    fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewProxyResp {
    proxy_name:  String,
    remote_addr: String,
    error:       String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MsgHeader {
    pub msg_type:   MsgType,
    pub len:        u64,
}

impl MsgHeader {
    pub fn new(msg_type: MsgType, len: u64) -> Self {
        Self {
            msg_type,
            len,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MsgType(u8);

#[allow(non_camel_case_types)]
pub const TypeLogin: MsgType        = MsgType('o' as u8);
pub const TypeLoginResp: MsgType    = MsgType('1' as u8);

pub const TypeNewProxy: MsgType     = MsgType('p' as u8);
pub const TypeNewProxyResp: MsgType = MsgType('2' as u8);
pub const TypeCloseProxy: MsgType   = MsgType('c' as u8);

pub const TypeNewWorkConn: MsgType      = MsgType('w' as u8);
pub const TypeReqWorkConn: MsgType      = MsgType('r' as u8);
pub const TypeStartWorkConn: MsgType    = MsgType('s' as u8);

pub const TypeNewVisitorConn: MsgType       = MsgType('v' as u8);
pub const TypeNewVisitorConnResp: MsgType   = MsgType('3' as u8);

pub const TypePing: MsgType    = MsgType('h' as u8);
pub const TypePong: MsgType    = MsgType('4' as u8);

pub const TypeUDPPacket: MsgType    = MsgType('u' as u8);

pub const TypeNatHoleVisitor: MsgType           = MsgType('i' as u8);
pub const TypeNatHoleClient: MsgType            = MsgType('n' as u8);
pub const TypeNatHoleResp: MsgType              = MsgType('m' as u8);
pub const TypeNatHoleClientDetectOK: MsgType    = MsgType('d' as u8);
pub const TypeNatHoleSid: MsgType               = MsgType('5' as u8);


pub const MSG_HEADER_SIZE: usize = 9;

pub fn msg_header_encode(hdr: &MsgHeader) -> [u8; MSG_HEADER_SIZE] {
    let mut buf = [0; MSG_HEADER_SIZE];
    buf[0] = hdr.msg_type.0;
    buf[1..MSG_HEADER_SIZE].copy_from_slice(&hdr.len.to_be_bytes());

    buf
}

pub fn msg_header_decode(buf: &[u8; MSG_HEADER_SIZE]) -> MsgHeader {
    MsgHeader {
        msg_type:   MsgType(buf[0]),
        len:        u64::from_be_bytes([buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8]]),
    }
}


