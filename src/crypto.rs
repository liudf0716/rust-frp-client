
use aes::cipher::{AsyncStreamCipher, KeyIvInit};
use ring::pbkdf2;
use std::num::NonZeroU32;
use anyhow::Result;

type Aes128CfbEnc = cfb_mode::Encryptor<aes::Aes128>;
type Aes128CfbDec = cfb_mode::Decryptor<aes::Aes128>;

static PBKDF2_ALG: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA1;
const  DEFAULT_SALT: &str    = "frp"; 

#[derive(Debug, Clone)]
pub struct FrpCoder {
    iv:     [u8; 16],
    key:    [u8; 16],
    salt:   String,
    token:  String,
    enc:    Aes128CfbEnc,
    dec:    Aes128CfbDec,
}

impl FrpCoder {

    pub fn new(token: String, iv: [u8; 16]) -> Self {
        let mut key = [0; 16];
        pbkdf2::derive(PBKDF2_ALG, NonZeroU32::new(64).unwrap(), DEFAULT_SALT.as_bytes(), token.as_bytes(), &mut key);

        Self{
            iv,
            key,
            salt: DEFAULT_SALT.to_string(),
            token,
            enc: Aes128CfbEnc::new(&key.into(), &iv.into()),
            dec: Aes128CfbDec::new(&key.into(), &iv.into()),
        }
    }

    pub fn key(&self) -> &[u8; 16] {
        &self.key
    }

    pub fn encypt(&mut self, buf: &mut Vec<u8>) -> Result<()> {
        let enc = self.enc.clone();
        enc.encrypt(buf);

        Ok(())
    } 

    pub fn decrypt(&mut self, buf: &mut Vec<u8>) -> Result<()> {
        let dec = self.dec.clone();
        dec.decrypt(buf);

        Ok(())
    }
}
