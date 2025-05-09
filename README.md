### Senec Client

### Repo Overview

#### Firmware
This contains the actual firmware for the soc with all the functionality.
This builds using a different toolchain. Please check the readme for [esp-idf-template](https://github.com/esp-rs/esp-idf-template) (which this was originally based  on) for more information regarding dependencies and tools you will need to compile this project.
Build and flash the firmware
```shell
cd firmware 
CRATE_CC_NO_DEFAULTS=1 cargo run -- --partition-table partition.csv
```
#### Display
This crate contains all the code for the display. It handels UI elements and defines an interface on how to use the display.
Both the firmware and the simulator make use of this crate. 

#### Simulator
This contains a simulator, which does not simulate the soc but the display.
This is used to rapidly prototype changes to the ui.
Both the actual firmware and the simulator make use of the /display crate that defines how content is displayed on the display.



Build and run the simulator:
```shell
cd simulator
cargo run
```




### Reproduce
Add .env file!

```shell
WIFI_PASS=
WIFI_SSID=
SERVER_ADDR=
```
SEVER_ADDR is the adress of the senec server inside your local network

Previous tracking of this repo happend over at: [prev repo](https://github.com/vuoz/senec-client)
This contains all the progress including the very first commit

Build on Linux/MacOS ( check dependencies of espflash etc.):
```shell
cargo run  -- --partition-table partition.csv
```
Build on Windows using wsl ( again check the dependencies of espflash etc.):
```shell
sh w.sh
```

### Parts used for this build
- [Arduino Nano Esp32](https://store.arduino.cc/products/nano-esp32)
- [E-Ink Display Waveshare 2.9inch ](https://www.waveshare.com/2.9inch-e-paper-module.htm)
- [1100mAh LiPo battery 3.7V](https://www.amazon.de/EEMB-Lithium-Wiederaufladbarer-Lipo-Akku-JST-Anschluss/dp/B08FD39Y5R)
- [TP4056 LiPo Charger](https://www.amazon.de/-/en/dp/B07XG5F9T3)
- [Simple on off switches](https://www.amazon.de/-/en/dp/B09QQKMWRR)

### PCB for faster assembly
Take the .zip file from /gerbers and upload it to any PCB manufacturer of you choice


### Assembled Version

![assembled](https://github.com/user-attachments/assets/8fb80f75-c8ca-481f-b833-c66c744cd7ce)




### Todos
- [ ] Add code to check battery percentage and update display
- [ ] Add code to check for charging status
