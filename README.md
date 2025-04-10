# EvaPanel

EvaPanel is a lightweight WebKit-based kiosk web-browser with remote controls,
focused on HMI (human-machine-interface) web-applications.

EvaPanel is developed for Linux. Building Windows binaries is possible, but
certain functions may not work properly.

![App](app_preview.png?raw=true)

## Features

* A solution for full-screen kiosk applications

* Works fine with most of web apps even on embedded systems with 512MB
  RAM (min. 300MB total system RAM is recommended)

* Can restrict navigation to allowed URLs only

* Remote-controlled

## Building

* [Install Rust](https://www.rust-lang.org/tools/install) and optionally
  [justfile](https://github.com/casey/just)

### Linux x86_64

```
just docker-image-x86_64 linux-x86_64
```

or corresponding commands from the `justfile`.

### Linux ARM64

```
just docker-image-aarch64 linux-aarch64
```

### Windows

No special instructions:

```
cargo build --release
```

## Installing a kiosk system

* install Linux and *xorg* with some lightweight window manager (e.g. *i3wm*)

* display control remote functions require *xrandr* and *xbacklight*

* if installing on another system, which had not been used for building,
  install the following system packages (the minor versions may differ):

```
apt install -y libwebkit2gtk-4.1-0
```

* create a user

* configure either graphical or console auto-login (recommended)

* in case of console auto-login, put the following into the user's home
  directory:

append to **~/.profile**

```
evapanel-launch.sh
```

* put **.xinitrc** into the user's home directory:

```
i3 &
evapanel
```

* put the compiled *evapanel* binary and *evapanel-launch.sh* script to e.g.
  */usr/local/bin/*

* put *evapanel.yml* to the user's home directory and edit the properties.

As an alternative, the program ca be started as a systemd service (make sure
that either X or Wayland is running).

## HMI apps integration

To transparently integrate a HMI application with EvaPanel, it must meet the
following requirements:

### Remote login/logout

The app must have JavaScript methods:

```javascript
$eva.hmi.login(user, password);
$eva.hmi.logout();
```

### Alerts

```javascript
$eva.hmi.display_alert(text, level, timeout);
```

where:

* text - a text to display
* level - info or warning
* timeout - an optional parameter, sets a timeout after which the alert is
  automatically closed

### Automatic WASM support

If [EVA ICS WebEngine](https://info.bma.ai/en/actual/eva-webengine/index.html)
WebAssembly extension is installed, the app can turn it on/off, based on the
kiosk's configuration:

```javascript
eva.wasm = config.wasm && (!window.navigator.userAgent.startsWith('EvaPanel ') || window.navigator.userAgent.search('/wasm ') > 0);
```

## Remote control

EvaPanel uses [BUS/RT](https://github.com/alttch/busrt) protocol.

EvaPanel can work in two modes: bus server and bus client. In case of client, a
local socket is opened (default: */tmp/evapanel.sock*)

Commands can be called with the default bus client, payload format is
MessagePack.

In server mode, the process registers itself as ".panel". In client mode, as
"eva.panel.HOSTNAME".

List of the available commands is provided in [*eapi.yml*](eapi.yml)

## Orchestrating

Available with [EVA ICS v4 HMI Kiosk manger
service](https://info.bma.ai/en/actual/eva4/svc/eva-kioskman.html).
