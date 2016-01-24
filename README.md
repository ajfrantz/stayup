A little program which creates a marker file when a TCP client is connected to
a specific port.

This application is mostly useless on its own, but the marker can be tested for
e.g. when combined with [MythTV](https://www.mythtv.org/)'s [ACPI
wakeup](https://www.mythtv.org/wiki/ACPI_Wakeup) feature--by simply holding a
TCP connection open a client can prevent the automatic shutdown of the backend.
