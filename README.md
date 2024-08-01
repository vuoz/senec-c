### Senec Client


Add .env file!

```shell
WIFI_PASS=
WIFI_SSID=
SERVER_ADDR=
```
SEVER_ADDR is the adress of the senec server inside your local network


Build on Linux/MacOS ( check dependencies of espflash etc.):
```shell
cargo run 

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

### Todos
- [ ] Add code to check battery percentage and update display
- [ ] Add code to check for charging status
