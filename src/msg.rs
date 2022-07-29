
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::env::consts;
use anyhow::Result;
use md5;
use chrono::Utc;
use yamux::{Stream}; 
use futures_util::io::{AsyncReadExt, AsyncWriteExt};

use crate::config::Config;

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct LoginResp {
    version:    String,
    run_id:     String,
    server_udp_port:    i32,
    error:      String,
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

        let mut buff = [0; 512];
        let n = main_stream.read(&mut buff).await?;
        let header = msg_header_decode(&buff[0..9].try_into().unwrap());
        let resp = std::str::from_utf8(&buff[MSG_HEADER_SIZE..(n as usize)]).unwrap();
        
        Ok(serde_json::from_str(resp)?)
    }

    fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

}

impl LoginResp {
    pub fn run_id(&self) -> String {
        self.run_id.clone()
    }

    pub fn error(&self) -> String {
        self.error.clone()
    }
}

fn get_privilege_key(timestamp: i64, auth_token: String) -> String {
    let seed = format!("{}{}", auth_token, timestamp);
    let digest = md5::compute(seed);

    format!("{:x}", digest)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MsgHeader {
    msg_type:   MsgType,
    len:        u64,
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


