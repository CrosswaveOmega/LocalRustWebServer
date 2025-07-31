# A Local Rust Web Server

A simple web server meant for accessing a Raspberry Pi
on my local network, and then running specific commands
on said rpi.

It uses a series of json and html files to define
and create endpoints.

## Current features
* Tail logs
* Front 

## Examples

Add this somewhere in jsonroutes
```json
{"function_type": "normal_page",
"route": "/help",
"title": "Help Page",
"body": "help.html"
}
```