# Container Auto Port Forwarding

A Rust implementation for port forwarding from a [devContainer](https://containers.dev) to the Host.
The goal is to provide a developer friendly experience similar to VS Code, but editor independent.

## Overview

This Feature is based on a Server Client Model, which needs the Server running on the Host.
When the Feature is added to the `.devcontainer.json` the Client inside the Container will reach out to the host.
Therefore, the Host needs to run beforehand. Currently, the Host needs to be started manually, and the Host only supports a single Container.
This will change in the Future. The Host automatically terminates after the Container closes the connection.

## How to use it

This workflow is based on the current state of the development and will hopefully get more user-friendly in the near future.

To get up and running you have to add the copy the `.devcontainer/` directory.
Update the `devcontainer.json` to your preferences and add the feature as a local feature, like below:

```json
{
  ...,
  "feature": {
    "./auto_forward": {}
  }
}
```

Currently, the feature is not yet published, therefore only the local variant is possible.
Furthermore, the Server running on the Host needs to be build and setup:

1. `git clone <repo>`
2. `cd autoForward`
3. `cargo run --release --bin host`

Sadly there are no prebuild binaries ready, therefore you will need Cargo to build your own.
Hope that will change fast, and I would love some feedback for further improvements.

## Documentation

The Following Documentation is more about the Application itself.
This might help, if you are interested in supporting that feature by contribution.

### Protocol

The Protocol is designed to be is slim and light way as possible.
Its function is to define the message size, type and destination.
Therefore, the Header containers 3 entries with the total size of 64 bit.

|Name|Size|Description|
|:-:|:-:|:-:|
|Message Size|32 bit|Specifies the Total Body size, which should be read after the header|
|Function|8 bit|The Field is multi use and details can be found in the Protocol Section|
|Port|16 bit| Port to forward |
|Reserved|8 bit| Unused |

The Body follows with the Message itself.

#### Function

Functions are defined in two sets, internal and external functions.
Internal functions should not be propagated by the Multiplexer, rather consumed.
Internal functions are there to manage the application State.
External Function are meant for the exchange between Container and Host.


|Bit Pattern|Name|Description|
|:-:|:-:|:-:|
|`0000 0001`| **CLOSE** | Close Connection and Terminate Program |
|`0000 0100`| **TCP** | Forward Message as TCP Packet |
|`0000 0010`| **UDP** | Forward Message as UDP Packet |
|`0000 1100`| **CREATE TCP** | Create TCP Listener |
|`0000 1010`| **CREATE UDP** | Create UDP Listener |
|`0001 0000`| **New Listener**| Notification for the Multiplexer (Not in use)|



### Operations

The Auto Port Forwarding Functions are based on a TCP Socket, which allows bidirectional traffic.
Traffic is handled by the two Programs, one running on the Host and one on the Client.
The Program on the Host is called Multiplexer, because he is multiplexing the traffic via channels to the corresponding ports or handler.
