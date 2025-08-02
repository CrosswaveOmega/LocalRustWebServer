# A Local Rust Web Server

A simple, low profile web server meant for accessing a Raspberry Pi
on a local network, and then running specific commands
on said Raspberry Pi.

It uses a series of json and html files to define
and create endpoints and return formatted html
on each endpoint.

## Basic Overview

Static HTML Website templates are loaded within `/templates`, and are all bound to an integer within `template_config.json`.

The configuration for each website route is loaded from the json files within `/json_routes`.

Each valid route object is formatted with the following values:
* `"function_type"`->The type of function this endpoint is.  
 * can (currently) be "normal_page", "run_command", or "get_logs"
* `"title"`-> the title of the route.
* `"description"`-> help description of the page.
* `"template_num"`-> the template number of the page.  Set to 0 by default.

### Templates.

Each "template" is just a html file that has a 'title' and 'body' substituted in.


## Current features
* Tail logs
* Frontend for an api proxy.

