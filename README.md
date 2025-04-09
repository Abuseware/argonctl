# Argon One v3 Controller
Simple Argon One v3 Raspberry Pi 5 case controller.

## Why
The original software from Argon40 Technologies is a mix of Bash and Python scripts,
customized exclusively for Debian-based systems.

Installation via `curl | bash` is a solution that should never happen.
Additionally, the scripts were written in a non-portable way and without a known license.

## Argond
If all you want is a simple daemon controlling the fan speed, all you need to do is run `argond`.
Optional parameters will allow you to configure its operation.

It requires a working system D-Bus session and the installation of the `xyz.abuseware.argond.conf`
file in its configuration folder (usually `/etc/dbus-1/system.d`).
The attached configuration only allows `argond` to be run as root or nobody.

```
Usage: argond [OPTIONS]

Options:
      --temp-low <TEMP_LOW>      Low temperature treshold
      --temp-high <TEMP_HIGH>    High temperature treshold
      --log-scale [<LOG_SCALE>]  Use logarithmic scaling instead of linear [possible values: true, false]
  -d, --daemon                   Forking to background with dropped privileges
  -u, --uid <UID>                User for dropped privileges
  -l, --log <LOG>                Log file
  -c, --config <CONFIG>          Configuration file
  -h, --help                     Print help
```

If you specify configuration file, it'll be stored on shutdown, and loaded on startup. TOML format.

## Argon

Argon is a program that allows changing operating parameters on the fly. Available parameters:

```
Usage: argon [OPTIONS]

Options:
      --temp-low <TEMP_LOW>    Low temperature treshold
      --temp-high <TEMP_HIGH>  High temperature treshold
      --log-scale <LOG_SCALE>  Use linear scaling instead of logarithmic [possible values: true, false]
      --exit                   Exit daemon
  -h, --help                   Print help
```

## Batteries not included
Currently, cooling control is very simple; decisions are made based on differences in averaged samples.
The algorithm prefers a rapid increase in speed and a slowed-down deceleration.

There is no possibility to program the IR receiver, OLED extension, or DAC.

After the cooling control module is completed, the remaining functions will be implemented.
However, I would like to emphasize immediately that I do not have OLED and DAC modules,
so they will not be tested. If you are interested in donating such items, please contact me.

## License
ISC license, details in the [file](LICENSE).
