use anyhow::{Result, Context};
use serialport::SerialPort;
use std::io::{self, Write};
use regex::Regex;
use binaryfile::BinaryWriter;

pub struct SerialComm {
    pub port: Box<dyn SerialPort>,
    pub logpath: String,
    pub monitor: bool,
}

fn truncate_before_newline(buffer: &mut Vec<u8>) {
    if let Some(pos) = buffer.iter().position(|&byte| byte == 0x0D || byte == 0x0A) {
        buffer.drain(..=pos);
    }
}

impl SerialComm {
    pub fn new(port_name: &str, speed: u32) -> Result<Self> {
        let port = serialport::new(port_name, speed)
            .open_native()
            .with_context(|| format!("Failed to open serial port: {}", port_name))?;

        Ok(Self {
            port: Box::new(port),
            logpath: "".to_string(),
            monitor: true,
        })
    }

    pub fn set_monitoring(&mut self, monitor: bool) {
        self.monitor = monitor;
    }

    pub fn set_logpath(&mut self, logpath: &str) -> Result<()> {
        self.logpath = logpath.to_string();

        // ファイルが作成できるかチェック
        BinaryWriter::new(&self.logpath)
            .with_context(|| format!("Failed to create log file: {}", self.logpath))?;

        Ok(())
    }

    pub fn write(&mut self, cmd: &str) -> Result<()> {
        self.port
            .write_all(cmd.as_bytes())
            .context("Failed to write to serial port")?;
        Ok(())
    }

    pub fn wait_for(&mut self, target: &str) -> Result<String> {
        self.port
            .set_timeout(std::time::Duration::from_secs(60))
            .context("Failed to set serial timeout")?;

        let mut linebuffer: Vec<u8> = Vec::new();
        let re = Regex::new(target)
            .with_context(|| format!("Invalid regex pattern: {}", target))?;

        loop {
            let ab = self.port.bytes_to_read()
                .context("Failed to get bytes_to_read")?;

            if ab > 0 {
                let mut buffer = vec![0; ab as usize];

                self.port.read_exact(&mut buffer)
                    .context("Failed to read from serial port")?;

                if self.monitor {
                    print!("{}", String::from_utf8_lossy(&buffer));
                    io::stdout().flush().ok();
                }

                if !self.logpath.is_empty() {
                    let mut fw = BinaryWriter::open(&self.logpath);
                    fw.write(&buffer);
                }

                linebuffer.extend(&buffer);
                truncate_before_newline(&mut linebuffer);

                let data = String::from_utf8_lossy(&linebuffer);

                if let Some(caps) = re.captures(&data) {
                    return Ok(caps[0].to_string());
                }
            }
        }
    }
}

pub fn get_portlist() -> Result<Vec<String>> {
    let ports = serialport::available_ports()
        .context("Failed to enumerate serial ports")?;

    Ok(ports.into_iter().map(|p| p.port_name).collect())
}

pub fn get_port() -> Result<String> {
    let portlist = get_portlist()?;

    match portlist.len() {
        1 => Ok(portlist[0].clone()),
        0 => Err(anyhow::anyhow!("No serial ports found")),
        _ => Err(anyhow::anyhow!("Multiple serial ports detected")),
    }
}
