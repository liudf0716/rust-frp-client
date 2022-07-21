use std::collections::HashMap;
use ini::{Ini, Properties};
use anyhow::Result;

#[derive(Debug, Clone)]
struct ClientCommonConfig {
    server_addr:    String,
    server_port:    u16,
    pool_count:     u32,
    tcp_mux:        bool,
    token:          String,
    heartbeat_interval: u32,
    heartbeat_timeout:  u32,
}

impl ClientCommonConfig {
    pub fn new() -> ClientCommonConfig {
        ClientCommonConfig {
            server_addr: "0.0.0.0".to_string(),
            server_port: 7000,
            pool_count: 1,
            tcp_mux:    true,
            token:      "".to_string(),
            heartbeat_interval: 30,
            heartbeat_timeout:  90,
        }
    }
}

#[derive(Debug, Clone)]
struct ClientTcpConfig {
    service_type:   String,
    local_ip:       String,
    local_port:     u16,
    remote_port:    u16,
}

impl ClientTcpConfig {
    pub fn new() -> ClientTcpConfig {
        ClientTcpConfig {
            service_type: "tcp".to_string(),
            local_ip: "127.0.0.1".to_string(),
            local_port: 0,
            remote_port: 0
        }
    }
}

#[derive(Debug, Clone)]
struct ClientWebConfig {
    service_type:   String,
    local_ip:       String,
    local_port:     u16,
    custom_domains: String,
}

impl ClientWebConfig {
    pub fn new(stype: String) -> ClientWebConfig {
        ClientWebConfig {
            service_type: stype,
            local_ip: "127.0.0.1".to_string(),
            local_port: 0,
            custom_domains: "".to_string()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    common:         ClientCommonConfig,
    tcp_configs:    HashMap<String, ClientTcpConfig>,
    web_configs:    HashMap<String, ClientWebConfig>,
}

impl Config {
    pub fn new() -> Self {
        let mut common: ClientCommonConfig = ClientCommonConfig::new(); 
        let mut tcp_configs: HashMap<String, ClientTcpConfig> = HashMap::new();
        let mut web_configs: HashMap<String, ClientWebConfig> = HashMap::new();

        Self {
            common,
            tcp_configs,
            web_configs,
        }
    }

    pub fn load_config(&mut self, config_file: &str) -> Result<()>{
        let i = Ini::load_from_file(config_file).unwrap();
        for (sec, prop) in i.iter() {
            if "common".eq(sec.unwrap()) {
                self.parse_common_config(sec.unwrap(), &prop);
            } else {
                self.parse_proxy_config(sec.unwrap(), &i, &prop);
            }
        }

        Ok(())
    }
    
    pub fn server_addr(&self) -> &str {
        &self.common.server_addr
    }

    pub fn server_port(&self) -> u16 {
        self.common.server_port
    }

    pub fn auth_token(&self) -> &str {
        &self.common.token
    }

    fn parse_common_config(&mut self, name: &str, prop: &Properties) -> Result<()> {
        for (k, v) in prop.iter() {
            match k {
                "server_addr" => self.common.server_addr = v.to_string(),
                "server_port" => self.common.server_port = v.parse::<u16>().unwrap(),
                "auth_token" => self.common.token = v.to_string(),
                "heartbeat_interval" => self.common.heartbeat_interval = v.parse::<u32>().unwrap(),
                "heartbeat_timeout" => self.common.heartbeat_timeout = v.parse::<u32>().unwrap(),
                "tcp_mux" => if v.eq(&"false".to_string()) { 
                    self.common.tcp_mux = false
                },
                _ => println!("dont support {}", k),
            }
        };

        Ok(())
    }

    fn parse_proxy_config(&mut self, name: &str, config: &Ini, prop: &Properties) -> Result<()> {
        let section = config.section(Some(name)).unwrap();
        let stype = section.get("type").unwrap();

        if stype.eq("tcp") {
            let mut tcp_proxy_config = ClientTcpConfig::new();

            for (k, v) in prop.iter() {
                match k {
                    "local_ip" => tcp_proxy_config.local_ip = v.to_string(),
                    "local_port" => tcp_proxy_config.local_port = v.parse::<u16>().unwrap(),
                    "remote_port" => tcp_proxy_config.remote_port = v.parse::<u16>().unwrap(),
                    "type" => (),
                    _ => println!("invalid key {}", k),
                }
            }

            self.tcp_configs.insert(name.to_string(), tcp_proxy_config);

        } else if stype.eq("http") || stype.eq("https") {
            let mut web_proxy_config = ClientWebConfig::new(stype.to_string());

            for (k, v) in prop.iter() {
                match k {
                    "local_ip" => web_proxy_config.local_ip = v.to_string(),
                    "local_port" => web_proxy_config.local_port = v.parse::<u16>().unwrap(),
                    "custom_domains" => web_proxy_config.custom_domains = v.to_string(),
                    "type" => (),
                    _ => println!("invalid key {}", k),
                }
            }

            self.web_configs.insert(name.to_string(), web_proxy_config);
        } else {
            println!("{} not support", stype);
        }
        
        Ok(())
    }

}
