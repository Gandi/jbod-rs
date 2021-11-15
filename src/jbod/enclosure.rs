/*-
 * SPDX-License-Identifier: BSD-2-Clause
 *
 * BSD 2-Clause License
 *
 * Copyright (c) 2021, Gandi S.A.S.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice, this
 *    list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

#[allow(non_snake_case)]
pub mod BackPlane {
    use std::fmt;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::process::{Command, Stdio};

    use crate::utils::helper::Util::{LSSCSI, SG_INQ, SG_SES};

    extern crate prettytable;
    extern crate subprocess;
    use prettytable::{color, format, Attr, Cell, Row, Table};

    #[derive(Debug)]
    pub struct Enclosure {
        pub slot: String,
        pub device_path: String,
        pub vendor: String,
        pub model: String,
        pub revision: String,
        pub serial: String,
    }

    #[derive(Debug)]
    pub struct EnclosureFan {
        /// The slot number provided by the JBOD
        pub slot: String,
        /// The device serial number
        pub serial: String,
        /// The name of the component provided by the JBOD.
        pub description: String,
        /// The slot position used by `sg_ses`.
        pub index: String,
        /// The RPM speed of the FAN.
        pub speed: i64,
        /// The JBOD can provide extra information about the FAN speed
        /// and it can be used to create alerts in the future.
        pub comment: String,
    }

    /// Creates the pretty table for the enclosure.
    fn create_enclosure_table() -> Table {
        let mut enclosure_table = Table::new();
        enclosure_table.set_format(*format::consts::FORMAT_NO_BORDER);
        enclosure_table.add_row(Row::new(vec![
            Cell::new("SLOT")
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color::BLUE)),
            Cell::new("DEVICE")
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color::BLUE)),
            Cell::new("VENDOR")
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color::BLUE)),
            Cell::new("MODEL")
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color::BLUE)),
            Cell::new("REVISION")
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color::BLUE)),
            Cell::new("SERIAL")
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color::BLUE)),
        ]));

        enclosure_table
    }

    /// Creates the pretty table for the FAN.
    pub fn create_fan_table() -> Table {
        let mut enclosure_table = Table::new();
        enclosure_table.set_format(*format::consts::FORMAT_NO_BORDER);
        enclosure_table.add_row(Row::new(vec![
            Cell::new("SLOT")
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color::BLUE)),
            Cell::new("IDENT")
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color::BLUE)),
            Cell::new("DESCRIPTION")
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color::BLUE)),
            Cell::new("STATUS")
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color::BLUE)),
            Cell::new("RPM")
                .with_style(Attr::Bold)
                .with_style(Attr::ForegroundColor(color::BLUE)),
        ]));

        enclosure_table
    }

    /// Implementation to print the enclosure table without deal with the table.
    impl fmt::Display for Enclosure {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut enclosure_table = create_enclosure_table();
            enclosure_table.add_row(Row::new(vec![
                Cell::new(&self.slot),
                Cell::new(&self.device_path),
                Cell::new(&self.vendor),
                Cell::new(&self.model),
                Cell::new(&self.revision),
                Cell::new(&self.serial),
            ]));

            enclosure_table.printstd();
            Ok(())
        }
    }

    /// Returns strings with vendor, ident, rev and serial of the enclosure.
    ///
    /// This function get information from a given enclosure.
    ///
    /// # Arguments
    ///
    /// * `device` - a string with the device path of the enclosure
    ///
    /// # Example
    /// ```
    /// let (vendor, ident, rev, serial) = get_enclosure_details("/dev/sg9".to_string());
    /// ```
    ///
    fn get_enclosure_details(device: String) -> (String, String, String, String) {
        let mut vendor = "NONE".to_string();
        let mut ident = "NONE".to_string();
        let mut rev = "NONE".to_string();
        let mut serial = "NONE".to_string();
        let sginq_cmd = Command::new(SG_INQ)
            .args(&[device])
            .output()
            .expect("Failed to sg_inq the device");
        let sginq_output = String::from_utf8_lossy(&sginq_cmd.stdout);

        for output in sginq_output.split('\n') {
            if output.contains("Vendor") {
                vendor = output
                    .replace("Vendor identification:", "")
                    .trim()
                    .to_string();
            }
            if output.contains("identification") {
                ident = output
                    .replace("Product identification:", "")
                    .trim()
                    .to_string();
            }
            if output.contains("revision") {
                rev = output
                    .replace("Product revision level:", "")
                    .trim()
                    .to_string();
            }
            if output.contains("serial") {
                serial = output.replace("Unit serial number:", "").trim().to_string();
            }
        }

        return (vendor, ident, rev, serial);
    }

    /// Returns the fan speed(RPM) and a message provided by the jbod with extra information
    /// about the FAN setup.
    ///
    /// # Arguments
    ///
    /// * `device_path` - The enclosure device
    /// * `fan_index` - The fan slot on the JBOD
    ///
    fn get_enclosure_fan_speed(device_path: &str, fan_index: &str) -> (i64, String) {
        let mut speed: i64 = 0;
        let mut comment: String = String::new();

        let index = format!("--index={}", &fan_index);
        let sg_ses_cmd = Command::new(SG_SES)
            .arg(index)
            .arg(&device_path)
            .output()
            .expect("Failed to get fan speed");
        let sg_ses_output = String::from_utf8_lossy(&sg_ses_cmd.stdout);
        let output_spl: Vec<&str> = sg_ses_output.split("\n").collect();
        for output in output_spl {
            if output.contains("speed") {
                let output_speed: Vec<&str> = output.split(",").collect();
                let _speed = output_speed[1]
                    .trim()
                    .chars()
                    .skip_while(|c| !c.is_digit(10))
                    .take_while(|c| c.is_digit(10))
                    .fold(None, |acc, c| {
                        c.to_digit(10).map(|b| acc.unwrap_or(0) * 10 + b)
                    });
                speed = _speed.unwrap().into();
                comment = output_speed[2].trim().to_string();
            }
        }
        return (speed, comment);
    }

    /// Returns a vector with the EnclosureFan structure for each FAN.
    ///
    /// This function parses the output of sg_ses and collects information from
    /// each FAN.
    ///
    pub fn get_enclosure_fan() -> Vec<EnclosureFan> {
        let mut enclosure_fan: Vec<EnclosureFan> = Vec::new();

        let enclosures = get_enclosure();
        for enclosure in enclosures.iter() {
            let cmd = format!("{} -j -ff {} | grep Cooling", SG_SES, enclosure.device_path);
            let cmd_run = subprocess::Exec::shell(cmd.to_string())
                .stream_stdout()
                .unwrap();
            let enc_fan = BufReader::new(cmd_run);
            for encf in enc_fan.lines() {
                let encf_u = encf.unwrap(); // Don't borrow memory
                let encf_split: Vec<&str> = encf_u.split("[").collect();
                if encf_split.len() > 1 {
                    let index_vec: Vec<&str> = encf_split[1].split("]").collect();
                    let _description = encf_split[0].trim();
                    let _index = index_vec[0];
                    if !_description.is_empty() && !_index.is_empty() {
                        let is_present =
                            enclosure_fan.iter().any(|c| c.index == _index.to_string() && c.serial == enclosure.serial);
                        if is_present == false {
                            let (speed, comment): (i64, String) =
                                get_enclosure_fan_speed(&enclosure.device_path, &_index);
                            enclosure_fan.push(EnclosureFan {
                                slot: enclosure.slot.clone(),
                                serial: enclosure.serial.clone(),
                                description: _description.to_string(),
                                index: _index.to_string(),
                                speed: speed,
                                comment: comment,
                            });
                        }
                    }
                }
            }
        }
        enclosure_fan
    }

    /// Returns a vector with the Enclosure structure for each enclosure.
    ///
    /// This function parses `lsscsi` and calls `get_enclosure_details` to full
    /// fill the Enclosure structure.
    ///
    pub fn get_enclosure() -> Vec<Enclosure> {
        let lsscsi_cmd = Command::new(LSSCSI)
            .args(&["-g"])
            .output()
            .expect("Failed to run get_enclosure()");
        let lsscsi_output = String::from_utf8_lossy(&lsscsi_cmd.stdout);
        let mut enclosure: Vec<Enclosure> = Vec::new();

        for p_output in lsscsi_output.split('\n') {
            if p_output.contains("enclosu") {
                let mut s_output: Vec<&str> = p_output.split(' ').collect();
                s_output.retain(|&content| !content.is_empty());

                let device_index = s_output.iter().position(|&r| r.contains("/dev/")).unwrap();
                let (_vendor, _ident, _rev, _serial) =
                    get_enclosure_details(s_output[device_index].to_string());
                enclosure.push(Enclosure {
                    slot: s_output[0].to_string().replace(&['[', ']'][..], ""),
                    device_path: s_output[device_index].to_string(),
                    vendor: _vendor,
                    model: _ident,
                    revision: _rev,
                    serial: _serial,
                });
            }
        }

        enclosure
    }
}
