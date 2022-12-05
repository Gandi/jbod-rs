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
pub mod DiskShelf {
    use clap::ArgMatches;
    use colored::*;
    use std::collections::HashMap;
    use std::fs;
    use std::io::{BufRead, BufReader};
    use std::process::{exit, Command, Stdio};

    use crate::jbod::enclosure::BackPlane;
    use crate::utils::helper::Util;
    use crate::utils::helper::Util::{SCSI_TEMP, SGINFO, SG_MAP};

    #[derive(Debug)]
    pub struct Disk {
        // Enclosure number identification, example: 15:0:1:0
        pub enclosure: String,
        // Disk slopt identification
        pub slot: String,
        // Disk path, example: /dev/sg105
        pub device_path: String,
        // Disk map, example: /dev/sdcz
        pub device_map: String,
        // Disk temperature
        pub temperature: String,
        // Disk vendor
        pub vendor: String,
        // Disk model
        pub model: String,
        // Disk serial number
        pub serial: String,
        // Disk firmware revision
        pub fw_revision: String,
        // Path to led control file
        pub led_locate_path: String,
        // Path to led control file
        pub led_fault_path: String,
    }

    /// Returns a string with the temperature
    ///
    /// This function is a wrapper over scsi_temperature script.
    ///
    /// # Argumets
    ///
    /// * `disk` - a string with the device path
    ///
    /// # Example
    /// ```
    /// let temperature = get_disk_temperature("/dev/sg100");
    /// ```
    ///
    fn get_disk_temperature(disk: String) -> String {
        let scsi_temp_cmd = Command::new(SCSI_TEMP)
            .args(&[disk])
            .output()
            .expect("Failed to scsi_temperature the device");
        let scsi_temp_output = String::from_utf8_lossy(&scsi_temp_cmd.stdout);
        let output_spl: Vec<&str> = scsi_temp_output.split('\n').collect();
        let temperature: String = output_spl[2].chars().filter(|n| n.is_digit(10)).collect();

        temperature
    }

    /// Returns a string with the disk firmware version
    ///
    /// This function is a wrapper over sginfo script.
    ///
    /// # Argumets
    ///
    /// * `disk` - a string with the device path
    ///
    /// # Example
    /// ```
    /// let fw_revision = get_disk_firmware("/dev/sg100");
    /// ```
    ///
    fn get_disk_firmware(disk: String) -> String {
        let mut fw_revision = String::new();
        let sginfo_temp_cmd = Command::new(SGINFO)
            .args(&[disk])
            .output()
            .expect("Failed to sginfo the device");
        let sginfo_temp_output = String::from_utf8_lossy(&sginfo_temp_cmd.stdout);

        for fw_info in sginfo_temp_output.split('\n') {
            if fw_info.contains("Revision level") {
                fw_revision = fw_info.replace("Revision level:", "").trim().to_string();
                break;
            }
        }

        fw_revision
    }
    /// Returns a string with the disk serial number
    ///
    /// # Arguments
    ///
    /// * `disk` - a string with the device path
    ///
    /// # Example
    /// ```
    /// let serial = get_disk_serial("/dev/sg100");
    /// ```
    ///
    fn get_disk_serial(disk: String) -> String {
        let res = fs::read(disk + "/vpd_pg80");
        let content = match res {
            Ok(c) => c.to_vec(),
            Err(_err) => "N/A".as_bytes().to_vec(),
        };

        unsafe { String::from_utf8_unchecked(content.to_vec()).to_string() }
    }

    /// Returns a string with the disk vendor
    ///
    /// # Arguments
    ///
    /// * `disk` - a string with the device path
    ///
    /// # Example
    /// ```
    /// let serial = get_disk_vendor("/dev/sg100");
    /// ```
    ///
    fn get_disk_vendor(disk: String) -> String {
        let res = fs::read(disk + "/vendor");
        let content = match res {
            Ok(c) => c.to_vec(),
            Err(_err) => "N/A".as_bytes().to_vec(),
        };

        unsafe {
            let mut s = String::from_utf8_unchecked(content.to_vec()).to_string();
            s.truncate(s.len() - 1);
            s
        }
    }

    /// Returns a string with the disk model
    ///
    /// # Arguments
    ///
    /// * `disk` - a string with the device path
    ///
    /// # Example
    /// ```
    /// let serial = get_disk_model("/dev/sg100");
    /// ```
    ///
    fn get_disk_model(disk: String) -> String {
        let res = fs::read(disk + "/model");
        let content = match res {
            Ok(c) => c.to_vec(),
            Err(_err) => "N/A".as_bytes().to_vec(),
        };

        unsafe {
            let mut s = String::from_utf8_unchecked(content.to_vec()).to_string();
            s.truncate(s.len() - 1);
            s
        }
    }

    /// Returns a HashMap with two strings, example: /dev/sg116 and /dev/sddk
    ///
    /// This function is a wraper over sg_map
    ///
    fn get_disk_sd_map() -> HashMap<String, String> {
        let mut disks: HashMap<String, String> = HashMap::new();
        let mut sg_map_cmd = Command::new(SG_MAP).stdout(Stdio::piped()).spawn().unwrap();

        {
            let stdout = sg_map_cmd.stdout.as_mut().unwrap();
            let stdout_reader = BufReader::new(stdout);
            let stdout_lines = stdout_reader.lines();

            for line in stdout_lines {
                let line_unwrap = line.unwrap().to_owned();
                let output_split: Vec<&str> = line_unwrap.split_whitespace().collect();
                if output_split.len() > 1 {
                    disks.insert(output_split[0].to_string(), output_split[1].to_string());
                } else {
                    disks.insert(output_split[0].to_string(), "NONE".to_string());
                }
            }
        }
        sg_map_cmd.wait().unwrap();

        disks
    }

    /// Returns a string with the led file location or NONE
    ///
    /// This function tries to localize if the enclosure provide led support via file
    ///
    /// # Arguments
    ///
    /// * `enclosure_slot` - a string reference with the enclosure_slot identification
    /// * `disk_slot` - a string reference with the disk slot number identification
    ///
    fn get_disk_led_locate_path(enclosure_slot: &str, disk_slot: &str) -> String {
        let sys_class_enclosure: &str = "/sys/class/enclosure/";

        Util::verify_sysclass_folder(sys_class_enclosure);

        let led_locate_path = sys_class_enclosure.to_string()
            + enclosure_slot
            + &"/".to_string()
            + &disk_slot.to_string()
            + &"/locate".to_string();

        if Util::path_exists(&led_locate_path) {
            return led_locate_path;
        } else {
            return "NONE".to_string();
        }
    }

    /// Returns a string with the led file location or NONE
    ///
    /// This function tries to localize if the enclosure provide led support via file
    ///
    /// # Arguments
    ///
    /// * `enclosure_slot` - a string reference with the enclosure_slot identification
    /// * `disk_slot` - a string reference with the disk slot number identification
    ///
    fn get_disk_led_fault_path(enclosure_slot: &str, disk_slot: &str) -> String {
        let sys_class_enclosure: &str = "/sys/class/enclosure/";

        Util::verify_sysclass_folder(sys_class_enclosure);

        let led_fault_path = sys_class_enclosure.to_string()
            + enclosure_slot
            + &"/".to_string()
            + &disk_slot.to_string()
            + &"/fault".to_string();

        if Util::path_exists(&led_fault_path) {
            return led_fault_path;
        } else {
            return "NONE".to_string();
        }
    }

    /// Here we write 0 or 1 into the disk led file
    fn set_disk_led_locate(disk: String, option: &str) {
        if Util::path_exists(&disk) {
            let jbod = jbod_disk_map();
            let found_disk: Vec<Disk> = jbod
                .into_iter()
                .filter(|v| (v.device_path == disk) || (v.device_map == disk))
                .collect();
            if !found_disk.is_empty() {
                if Util::path_exists(&found_disk[0].led_locate_path) {
                    fs::write(&found_disk[0].led_locate_path, option.clone())
                        .expect("Unable to write on locate led");
                    match option {
                        "0" => {
                            println!(
                                "Disk slot: {} {}",
                                found_disk[0].slot.green().bold(),
                                option
                            );
                        }
                        "1" => {
                            println!(
                                "Disk slot: {} {}",
                                found_disk[0].slot.yellow().blink().bold(),
                                option
                            );
                        }
                        _ => println!("Option not identified"),
                    }
                } else {
                    println!(
                        "{}: {} does not expose locate led",
                        "Error".red().bold(),
                        disk.yellow().bold()
                    );
                }
            }
        } else {
            println!(
                "{} device {} not found",
                "Error:".red().bold(),
                disk.yellow().bold(),
            );
            exit(1);
        }
    }

    /// Here we write 0 or 1 into the disk led file
    fn set_disk_led_fault(disk: String, option: &str) {
        if Util::path_exists(&disk) {
            let jbod = jbod_disk_map();
            let found_disk: Vec<Disk> = jbod
                .into_iter()
                .filter(|v| (v.device_path == disk) || (v.device_map == disk))
                .collect();
            if !found_disk.is_empty() {
                if Util::path_exists(&found_disk[0].led_fault_path) {
                    fs::write(&found_disk[0].led_fault_path, option.clone())
                        .expect("Unable to write on locate led");
                    match option {
                        "0" => {
                            println!(
                                "Disk slot: {} {}",
                                found_disk[0].slot.green().bold(),
                                option
                            );
                        }
                        "1" => {
                            println!(
                                "Disk slot: {} {}",
                                found_disk[0].slot.red().blink().bold(),
                                option
                            );
                        }
                        _ => println!("Option not identified"),
                    }
                } else {
                    println!(
                        "{}: {} does not expose fault led",
                        "Error".red().bold(),
                        disk.yellow().bold()
                    );
                }
            }
        } else {
            println!(
                "{} device {} not found",
                "Error:".red().bold(),
                disk.yellow().bold(),
            );
            exit(1);
        }
    }

    /// Returns strings with enclouse, slot, device path, temperature and the location of
    /// the led's files
    ///
    /// This function gets all details of a disk.
    ///
    /// # Arguments
    ///
    /// * `device` - a string with the device path
    /// * `enclosure_slot` - the enclosure slot number, example: 15:0:0:0
    ///
    fn get_disk_details(
        device: String,
        enclosure_slot: String,
    ) -> (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    ) {
        let sys_class_enclosure: &str = "/sys/class/enclosure/";
        let mut enclosure = String::new();
        let mut slot = String::new();
        let mut device_path = String::new();
        let mut temperature = String::new();
        let mut fw_revision = String::new();
        let mut vendor = String::new();
        let mut model = String::new();
        let mut serial = String::new();
        let mut disk_locate_led = String::new();
        let mut disk_fault_led = String::new();

        let path_tostr_spl: Vec<&str> = device.split('/').collect();
        let _slot = path_tostr_spl[5];
        let cmp_slot = _slot.to_lowercase();

        Util::verify_sysclass_folder(sys_class_enclosure);

        if cmp_slot.contains("slot")
            || cmp_slot.contains("disk")
            || cmp_slot.contains("array device")
            || cmp_slot.bytes().all(|c| c.is_ascii_digit())
        {
            let generic_device = format!("{sys_class_enclosure}{enclosure_slot}/{_slot}/device");
            let physical_device = format!("{generic_device}/scsi_generic/");

            if Util::path_exists(&physical_device) {
                let physical_path = fs::read_dir(physical_device).unwrap();
                for dev in physical_path {
                    let _dev = dev.unwrap().path();
                    let __dev = _dev.to_str().unwrap();
                    let split_dev: Vec<&str> = __dev.split('/').collect();
                    let __get_slot: Vec<&str> = split_dev[5].split(',').collect();
                    enclosure = split_dev[4].to_string();
                    slot = __get_slot[0].to_string();
                    device_path = format!("/dev/{}", split_dev[8]);
                    temperature = get_disk_temperature(device_path.clone());
                    fw_revision = get_disk_firmware(device_path.clone());
                    vendor = get_disk_vendor(generic_device.clone().to_string());
                    model = get_disk_model(generic_device.clone().to_string());
                    serial = get_disk_serial(generic_device.clone().to_string());
                    disk_locate_led = get_disk_led_locate_path(&enclosure_slot, split_dev[5]);
                    disk_fault_led = get_disk_led_fault_path(&enclosure_slot, split_dev[5]);
                }
            }
        }
        (
            enclosure,
            slot,
            device_path,
            temperature,
            fw_revision,
            vendor,
            model,
            serial,
            disk_locate_led,
            disk_fault_led,
        )
    }

    /// Returns a vector of disk structure
    ///
    /// This function collects all information of a disk
    ///
    /// # Arguments
    ///
    /// * `enc_vec` - A vector including all enclosures we want to scan for disks.
    ///
    fn get_disks_per_enclosure(enc_vec: Vec<BackPlane::Enclosure>) -> Vec<Disk> {
        let mut disk: Vec<Disk> = Vec::new();
        let sys_class_enclosure: &str = "/sys/class/enclosure/";
        let sg_map = get_disk_sd_map(); // Get all sg_map once in a HashMap

        Util::verify_sysclass_folder(sys_class_enclosure);

        for enclosure in enc_vec {
            let paths = fs::read_dir(sys_class_enclosure.to_string() + &enclosure.slot).unwrap();
            for path in paths {
                let _get_path = path.unwrap().path();

                let path_tostr = _get_path.to_str().unwrap();
                let (
                    _enclosure,
                    _slot,
                    _device_path,
                    _temperature,
                    _fw_revision,
                    _vendor,
                    _model,
                    _serial,
                    _led_locate_path,
                    _led_fault_path,
                ) = get_disk_details(path_tostr.to_string(), enclosure.slot.to_string());

                if !_device_path.is_empty() {
                    disk.push(Disk {
                        enclosure: _enclosure,
                        slot: _slot,
                        device_map: sg_map.get(&_device_path).unwrap().to_string(),
                        device_path: _device_path,
                        temperature: _temperature,
                        fw_revision: _fw_revision,
                        vendor: _vendor,
                        model: _model,
                        serial: _serial,
                        led_locate_path: _led_locate_path,
                        led_fault_path: _led_fault_path,
                    });
                }
            }
        }

        disk
    }

    /// Returns a vector with disk structure
    ///
    /// This is the public function that returns all disks and its information.
    ///
    pub fn jbod_disk_map() -> Vec<Disk> {
        let enc = BackPlane::get_enclosure();
        let disks = get_disks_per_enclosure(enc);

        disks
    }

    /// [TODO] fix the return
    ///
    /// This function handles the disk led manipulation
    ///
    /// # Arguments
    ///
    /// * `options` - a reference of ArgMatches
    ///
    pub fn jbod_led_switch(options: &ArgMatches) -> Result<(), ()> {
        let is_locate = options.is_present("locate");
        let is_fault = options.is_present("fault");
        let on = options.is_present("on");
        let off = options.is_present("off");

        if on && off {
            println!(
                "Not christmas yet {}{}{}!",
                ":".green().bold(),
                "_".yellow().bold().blink(),
                ")".red().bold()
            );
            exit(1);
        }

        if is_locate {
            let disk = options
                .value_of("locate")
                .unwrap_or(&"/dev/null".to_string())
                .to_string();
            if on {
                set_disk_led_locate(disk.clone(), &"1".to_string());
            }
            if off {
                set_disk_led_locate(disk.clone(), &"0".to_string());
            }
        }

        if is_fault {
            let disk = options
                .value_of("fault")
                .unwrap_or(&"/dev/null".to_string())
                .to_string();
            if on {
                set_disk_led_fault(disk.clone(), &"1".to_string());
            }
            if off {
                set_disk_led_fault(disk.clone(), &"0".to_string());
            }
        }

        Ok(())
    }
}
