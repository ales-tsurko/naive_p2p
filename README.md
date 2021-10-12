naive_p2p
=========

A very basic peer-to-peer network sending a message between peers.




## Usage

Run multiple instances on the same machine (not sure it works between different
machines due the byte ordering).

```
naive_p2p 0.1.0
Ales Tsurko <ales.tsurko@gmail.com>
Naive implementation of a peer-to-peer network.

USAGE:
    main [OPTIONS] --message <STRING> --period <SECONDS> --port <NUMBER>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --connect <ADDRESS>    Address of another peer to connect
    -m, --message <STRING>     Sets the message, which will be send by this peer
    -t, --period <SECONDS>     Sets the period between sending the message
    -p, --port <NUMBER>        Sets the port number
```
