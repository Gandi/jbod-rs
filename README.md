# jbod - Generic storage enclosure tool

<b>jbod</b> is a CLI that allows us to get information from disk enclosures and JBOD as well as control the led identification of disks. It also comes with a prometheus-exporter where we can collect metrics like the disks temperature, number of fans, the RPM of the fans and etc.

It was based on [encled](https://github.com/r5r3/encled) and [WDDCS](https://documents.westerndigital.com/content/dam/doc-library/en_us/assets/public/western-digital/product/platforms/ultrastar-data60-hybrid-platform/user-guide-ultrastar-data60.pdf) tool.

### Commands:
* <b>```jbod help```</b> - Help menu
* <b>```jbod list [-e|--enclosure]```</b> - Provide a storage enclosure overview
* <b>```jbod list [-d|--disks]```</b> - List all disks per enclosure 
* <b>```jbod list [-f|--fan]```</b> - List all FAN on the jbod
* <b>```jbod prometheus [-i|--ip-address][-p|--port]```</b> - Start prometheus-exporter 
* <b>```jbod led [-l|--locate] <device> --[on|off]```</b> - Turn ON/OFF disk bay location led.
* <b>```jbod led [-f|--fault] <device> --[on|off]```</b> - Turn ON/OFF disk bay fault led.

### Example of usage:
![jbod-cli](https://raw.githubusercontent.com/Gandi/jbod-rs/release/gif/jbod.gif)
 
### Grafana:
![graphana](https://raw.githubusercontent.com/Gandi/jbod-rs/release/gif/jbod-exporter.gif)

### Build the project:
* Release: <b>```cargo build --release```</b>

### Debian package:
* First install: <b>```cargo install cargo-deb```</b>
* Generate the debian package: <b>```cargo deb -v```</b>

### Crate:
[https://crates.io/crates/jbod](https://crates.io/crates/jbod)

### Contributing:
<a href="https://github.com/Gandi/jbod-rs/graphs/contributors">
  <img src="https://contributors-img.web.app/image?repo=Gandi/jbod-rs" />
</a>

### License:

The project is made available under the BSD 2-Clause license. See the `LICENSE` file for more information.
