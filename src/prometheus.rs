/*-
 * SPDX-License-Identifier: BSD-2-Clause
 *
 * BSD 2-Clause License
 *
 * Copyright (c) 2021-2023, Gandi S.A.S.
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

#[macro_use]
extern crate lazy_static;

use std::net::SocketAddr;

use std::env;
use std::result::Result;
use warp::{Filter, Rejection, Reply};
use prometheus::{
    IntGauge, IntGaugeVec, Opts, Registry,
};

mod jbod;
mod utils;
use crate::jbod::disks::DiskShelf;
use crate::jbod::enclosure::BackPlane;
use crate::utils::helper::Util;

// Declare code to be executed at runtime, this includes anything requiring
// heap allocations and function calls to be computed.
//
// Every exporter metrics are declared here first.
//
lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    pub static ref NUMBER_OF_ENCLOSURES: IntGauge =
        IntGauge::new("number_of_enclosures", "Number of enclosures").expect("metric can be created");

    pub static ref JBOD_SLOT_TEMPERATURE: IntGaugeVec =
        IntGaugeVec::new(
        Opts::new("jbod_slot_temperature", "Enclosure number, slot position and temperature"),
        &["slot", "enclosure"]
    ).expect("metric can be created");

    pub static ref JBOD_FAN_RPM: IntGaugeVec =
        IntGaugeVec::new(
        Opts::new("jbod_fan_rpm", "The RPM speed of FAN components, device and slot"),
        &["device", "slot"]
    ).expect("metric can be created");
}

/// Here we register the metrics, this function is called in the `main()`.
fn register_metrics() {
    REGISTRY.register(Box::new(NUMBER_OF_ENCLOSURES.clone()))
        .expect("collector can be registered");
    REGISTRY.register(Box::new(JBOD_SLOT_TEMPERATURE.clone()))
        .expect("collector can be registered");
    REGISTRY.register(Box::new(JBOD_FAN_RPM.clone()))
        .expect("collector can be registered");
}

// Index handler.
async fn index_handler() -> Result<impl Reply, Rejection> {
    Ok("")
}

/// Returns an `i64` with the total number of enclosures.
async fn number_of_enclosure_metrics() -> i64 {
    let enclosure = BackPlane::get_enclosure();
    return(enclosure.len() as i64)
}

/// Returns Result with Reply and Rejection.
///
/// This function updates the prometheus-exporter metrics.
/// Also here we can find the logic behind each metric.
async fn metrics_handler() -> Result<impl Reply, Rejection> {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();

    // Enclosure FAN rpm
    let mut enclosure_fan = BackPlane::get_enclosure_fan();
    enclosure_fan.sort_by_key(|f| f.index.clone());
    for fan in enclosure_fan.iter() {
        JBOD_FAN_RPM.with_label_values(&[&fan.description, &fan.index])
            .set(fan.speed);
    }
    drop(enclosure_fan);

    // Enclosures
    let enclosures = number_of_enclosure_metrics();
    NUMBER_OF_ENCLOSURES.set(enclosures.await);

    // Disks slot temperature
    let mut disks_temperature = DiskShelf::jbod_disk_map();
    disks_temperature.sort_by_key(|d| d.slot.clone());
    for disk in disks_temperature.iter() {
        match disk.temperature.parse() {
            Ok(temperature) => {
                JBOD_SLOT_TEMPERATURE
                .with_label_values(&[&disk.slot, &disk.enclosure])
                .set(temperature)},
            Err(e) => eprintln!("Failed to read temperature: {:?} of disk: {:?}", e, disk),
        }
    }
    drop(disks_temperature);

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
        eprintln!("could not encode custom metrics: {}", e);
    };

    let mut res = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("custom metrics could not be from_utf8: {}", e);
            String::default()
        }
    };
    buffer.clear();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
        eprintln!("could not encode prometheus metrics: {}", e);
    };
    let res_custom = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("prometheus metrics could not be from_utf8'd: {}", e);
            String::default()
        }
    };
    buffer.clear();

    res.push_str(&res_custom);
    Ok(res)
}

/// `main()` function that starts the webserver.
#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let mut port: String = "9945".to_string();
    let mut ipv4: String = "0.0.0.0".to_string();

    if args.len() > 2 {
        if Util::is_string_numeric(&args[2]) {
            port = args[2].to_string();
        } else {
            println!("Port is not decimal, using default {}", port);
        }

        let t = args[1]
            .split(".")
            .map(Util::is_string_numeric)
            .any(|i| i == false);
        if !t {
            ipv4 = args[1].to_string();
        } else {
            println!("Using default ipv4: {}", ipv4);
        }
    }

    let adr: String = ipv4 + ":" + &port;
    let adr_convert: SocketAddr = adr.parse().expect("Could not parse SocketAddr");

    register_metrics();

    let metrics_route = warp::path!("metrics").and_then(metrics_handler);
    let route = warp::path::end().and_then(index_handler);

    println!("==> Started on {}", adr);
    warp::serve(metrics_route.or(route))
        .run(adr_convert)
        .await;
}
