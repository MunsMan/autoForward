# Container Auto Port Forwarding

The Purpose of this tool is to automatically forward ports from a devContainer to the Host.
This is currently planned as a feature because it needs to add a Service to the devContainer.


## Overview

This feature works by forwarding the traffic over a UNIX Socket, which is mounted into the container.
The goal is to work IDE independent and with minimal dependencies.
UNIX Sockets provide a fast data transfer and fast response time, but need some additional logic for better handling.
A basic protocol was introduced for organizing and directing traffic, which definition follows below. 


## Protocol

The Protocol is designed to be is slim and light way as possible.
Its function is to define the message size, type and destination.
Therefore, the Header containers 3 entries with the total size of 64 bit.

|Name|Size|Description|
|:-:|:-:|:-:|
|Message Size|32 bit|Specifiys the Total Body size, which should be read after the header|
|Function|8 bit|The Field is multi use and details can be found in the Protocol Section|
|Port|16 bit| Port to forward |
|Reserved|8 bit| Unused |

The Body follows with the Message itself.

### Function

Functions are defined in two sets, internal and external functions.
Internal functions should not be propergated by the Multiplexer, rather consumed.
Internal functions are there to manage the application State.
External Function are ment for the exchange between Container and Host.


|Bit Pattern|Name|Description|
|:-:|:-:|:-:|
|`0000 0001`| **CLOSE** | Close Connection and Terminate Program |
|`0000 0100`| **TCP** | Forward Message as TCP Packet |
|`0000 0010`| **UDP** | Forward Message as UDP Packet |
|`0000 1100`| **CREATE TCP** | Create TCP Listener |
|`0000 1010`| **CREATE UDP** | Create UDP Listener |
|`0001 0000`| **New Listener**| Notification for the Multiplexer |



## Operations

The Auto Port Forwarding Functions are based on a UNIX Socket, which allows non-blocking bidirectional traffic.
Traffic is handelt by the two Programms, one running on the Host and one on the Client.

### Host

The Host Programm needs to create Listener on the Host, which listen for incoming Requests.
