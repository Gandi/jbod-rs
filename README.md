# jbod - Generic storage enclosure tool

<b>jbod</b> is a tool aimed to provide basic commands to collect information from standard storage enclosures.

It was based on [encled](https://github.com/r5r3/encled) and [WDDCS](https://documents.westerndigital.com/content/dam/doc-library/en_us/assets/public/western-digital/product/platforms/ultrastar-data60-hybrid-platform/user-guide-ultrastar-data60.pdf) tool.

Ticket: https://phabricator.corp.gandi.net/T88239

### Commands:
* <b>```jbod help```</b> - Help menu
* <b>```jbod list [-e|--enclosure]```</b> - Provide a storage enclosure overview
* <b>```jbod list [-d|--disks]```</b> - List all disks per enclosure 
* <b>```jbod list [-f|--fan]```</b> - List all FAN on the jbod
* <b>```jbod prometheus [-i|--ip-address][-p|--port]```</b> - Start prometheus-exporter 
* <b>```jbod led [-l|--locate] <device> --[on|off]```</b> - Turn ON/OFF disk bay location led.
* <b>```jbod led [-f|--fault] <device> --[on|off]```</b> - Turn ON/OFF disk bay fault led.

### Example of usage:
[gif]: https://gitlab.corp.gandi.net/devops/jbod-rs/raw/master/gif/jbod.gif
![example][gif]

### Debian package:
* First install: <b>```cargo install cargo-deb```</b>
* Generate the debian package: <b>```cargo deb -v```</b>
