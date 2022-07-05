# EvaPanel

EvaPanel is a lightweight WebKit-based kiosk web-browser with remote controls,
focused on HMI (human-machine-interface) web-applications.

EvaPanel works fine with most of web apps even on embedded systems with <512MB
RAM (min. 400MB of RAM is recommended).

EvaPanel development is focused on Linux. Building Windows binaries is
possible, but certain functions may not work properly.

## Building

It is quite hard to compile a static binary with WebKit embedded. Also,
depending on system WebKit libraries makes much easier to apply updates, so

* Install Rust

* Build EvaPanel for your Linux distribution:

```
./build.sh
```

## Installing a kiosk system

* install Linux and *xorg* with some lightweight window manager (e.g. *i3wm*)

* display control remote functions require *xrandr* and *xbacklight*

* create a user

* configure either graphical or console auto-login (recommended)

* in case of console auto-login, put the following into user's home directory:

append to **~/.profile**

```
evapanel-launch.sh
```

**.xinitrc**

```
i3 &
evapanel
```

* put compiled *evapanel* binary and evapanel-launch.sh script to e.g.
  */usr/local/bin/*

* put *evapanel.yml* to the user's home directory and edit the properties.

## Remote control

EvaPanel uses [BUS/RT](https://busrt.bma.ai/) protocol.

EvaPanel can work in two modes: bus server and bus client. In case of client, a
local socket is opened (default: */tmp/evapanel.sock*)

Commands can be called with the default bus client, payload format is
MessagePack.

List of the available commands is provided in *eapi.yml*.
