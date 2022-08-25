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

#[forbid(unsafe_code)]
use clap::{App, Arg, ArgMatches, SubCommand};
use colored::*;
use nix::{
    sys::wait::waitpid,
    unistd::{fork, ForkResult},
};
use std::process::{exit, Command};

extern crate prettytable;
use prettytable::{Cell, Row};

mod jbod;
mod utils;
use crate::jbod::disks::DiskShelf;
use crate::jbod::enclosure::BackPlane;
use crate::utils::helper::Util;

/// Fallback help function, we should never fall here
fn help() {
    println!("Use command with help option");
}

/// Given a string representing a temperature like: [0-9]+ it will
/// return colored string first for the temperature second for the unit.
///
/// Coloration:
///
/// - Bellow 50 it's all green
/// - Between 45 excluded and below 50 included it's yellow bold
/// - Above it's blinking red you must act maybe :)
///
/// If temperature is not readable it return `None` it's caller responsibility
/// to report it properly.
///
fn color_temp(temperature: &str) -> Option<(ColoredString, ColoredString)> {
    let temp_conv = temperature.parse::<i32>().ok()?;
    let coloreds = if temp_conv > 45 && temp_conv <= 50 {
        (temperature.yellow().bold(),
        "c".yellow().bold())
    } else if temp_conv > 50 {
        (temperature.red().bold().blink(), "c".red().bold().blink())
    } else {
        (temperature.green(), "c".green())
    };
    Some(coloreds)
}

/// TODO: Rework error handling, perhaps we don't need return Result
///
/// Returns an empty Result for now.
///
/// This function is used in the `list` menu option,
/// it combines the options for list the enclosures,
/// disks and fan from the JBOD.
///
/// # Arguments
///
/// * `option` - clappy's ArgMatches
///
fn enclosure_overview(option: &ArgMatches) -> Result<(), ()> {
    let disks_option = option.is_present("disks");
    let enclosure_option = option.is_present("enclosure");
    let fan_option = option.is_present("fan");

    // If the options `-ed` or `-d` are used, it shows
    // the enclosure and disks altogether.
    if enclosure_option && disks_option || disks_option {
        let enclosure = BackPlane::get_enclosure();
        let mut disks = DiskShelf::jbod_disk_map();
        disks.sort_by_key(|d| d.slot.clone());
        for enc in enclosure {
            print!("{}", enc);
            println!("     '");
            for disk in &disks {
                if enc.slot == disk.enclosure {
                    print!("     `+-");
                    print!(" Disk: {:<10}", disk.device_path.green(),);
                    if disk.device_map == "NONE" {
                        print!(" Map: {:<10}", disk.device_map.yellow());
                    } else {
                        print!(" Map: {:<10}", disk.device_map.green());
                    }
                    print!(" Slot: {:<10}", disk.slot.green());
                    print!(" Vendor: {:<10}", disk.vendor.blue());
                    print!(" Model: {:<10}", disk.model.blue());
                    print!(" Serial: {:<10} ", disk.serial.blue());
                    match color_temp(&disk.temperature) {
                        Some((temp_colored, unit_colored)) => print!("Temp: {}{:<2}", temp_colored, unit_colored),
                        None => print!("Temp: {:<4}", "ERR".red().bold().blink()),
                    }
                    println!(" Fw: {}", disk.fw_revision.blue());
                }
            }
        }
    // Here it shows only the enclosures.
    } else if enclosure_option && !disks_option {
        let enclosure = BackPlane::get_enclosure();
        for enc in enclosure {
            print!("{}", enc);
        }
    // Here it shows the FAN.
    } else if fan_option {
        let enclosure_fan = BackPlane::get_enclosure_fan();
        let mut fan_table = BackPlane::create_fan_table();
        for fan in enclosure_fan {
            fan_table.add_row(Row::new(vec![
                Cell::new(&fan.slot),
                Cell::new(&fan.index),
                Cell::new(&fan.description),
                Cell::new(&fan.comment),
                Cell::new(&fan.speed.to_string()),
            ]));
        }
        fan_table.printstd();
    }

    Ok(())
}

/// TODO: Rework error handling, perhaps we don't need return Result 
///
/// Returns an empty Result for now.
///
/// This function forks another binary for the prometheus-exporter. 
///
/// # Arguments
///
/// * `option` - clappy's ArgMatches
///
fn fork_prometheus(option: &ArgMatches) -> Result<(), ()> {
    let mut default_port = "9945";
    let mut default_address = "0.0.0.0";

    if let Some(port) = option.value_of("port") {
        default_port = port;
    }

    if let Some(ip) = option.value_of("ip-address") {
        default_address = ip;
    }

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            println!("prometheus-exporter pid: {:?}", child);
            waitpid(Some(child), None).unwrap();
            exit(0);
        }

        Ok(ForkResult::Child) => {
            Command::new(Util::JBOD_EXPORTER)
                .args(&[default_address, default_port])
                .spawn()
                .expect("Failed to spawn the target process");
            exit(0);
        }
        Err(_) => println!("Fork Failed"),
    }

    Ok(())
}

/// The main function that creates the menu.
fn main() {
    Util::verify_binary_needed();

    let matches = App::new("jbod")
        .version("0.0.1")
        .author("\nAuthor: Marcelo Araujo <marcelo.araujo@gandi.net>")
        .about("About: A generic storage enclosure tool")
        .subcommand(
            SubCommand::with_name("list")
                .about("list")
                .arg(
                    Arg::with_name("enclosure")
                        .short("e")
                        .long("enclosure")
                        .multiple(false)
                        .required(false)
                        .takes_value(false)
                        .help("List enclosure"),
                )
                .arg(
                    Arg::with_name("disks")
                        .short("d")
                        .long("disks")
                        .multiple(false)
                        .required(false)
                        .takes_value(false)
                        .help("List disks"),
                )
                .arg(
                    Arg::with_name("fan")
                        .short("f")
                        .long("fan")
                        .multiple(false)
                        .required(false)
                        .takes_value(false)
                        .help("List fan"),
                ),
        )
        .subcommand(
            SubCommand::with_name("led")
                .about("led")
                .arg(
                    Arg::with_name("locate")
                        .short("l")
                        .long("locate")
                        .required(false)
                        .multiple(true)
                        .value_name("DEVICE")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("fault")
                        .short("f")
                        .long("fault")
                        .required(false)
                        .multiple(true)
                        .value_name("DEVICE")
                        .takes_value(true),
                )
                .arg(Arg::with_name("on").long("on").required(false))
                .arg(Arg::with_name("off").long("off").required(false)),
        )
        .subcommand(
            SubCommand::with_name("prometheus")
                .about("Prometheus")
                .arg(
                    Arg::with_name("port")
                        .short("p")
                        .long("port")
                        .required(false)
                        .value_name("PORT")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("ip-address")
                        .short("ip")
                        .long("ip-address")
                        .required(false)
                        .value_name("IPADDRESS")
                        .takes_value(true),
                ),
        )
        .get_matches();

    // Here it matches the menu options with its respective functions.
    match matches.subcommand() {
        ("list", Some(m)) => enclosure_overview(m),
        ("led", Some(m)) => DiskShelf::jbod_led_switch(m),
        ("prometheus", Some(m)) => fork_prometheus(m),
        _ => Ok(help()),
    };
}
