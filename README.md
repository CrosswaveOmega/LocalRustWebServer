
---

# Local Rust Web Server

A lightweight, minimal-profile web server designed for use on devices connected on a local network (e.g., a Raspberry Pi). 

It provides access to web endpoints configurable within json files.

The web endpoints can return static formatted HTML page templates, execute commands on the other local device, and more to come.


Very much a work in progress.

---

## Overview

* HTML templates are stored in the `/templates` directory and are associated with integer keys defined in `template_config.json`.
* Each API route is defined by a JSON file in the `/json_routes` directory.
* Routes are mapped at startup and exposed as web endpoints.
* Dynamically formatted `/help` page.

---

## Route Configuration

Uses a combination of JSON and HTML files to define dynamic routes and serve formatted HTML content.

Each JSON route file can define one or many route objects, provided they have the following fields:

| Field           | Description                                                                                                   |
| --------------- | ------------------------------------------------------------------------------------------------------------- |
| `function_type` | The behavior type of the route. Supported types: `"normal_page"`, `"run_command"`, `"get_logs"`, `"call_api"` |
| `route`         | The url route for this particular page.  Required.                                                            |
| `title`         | The display title of the page.                                                                                |
| `description`   | A short help description of the endpoint, meant for use on the `/help` page.                                  |
| `template_num`  | The template number to use. Defaults to `0` if not specified.                                                 |
| `help_order`    | What order should this route be on the help page?  Defaults to `256` by default                               |
---

## Templates

Each template is a standalone HTML file that uses substitution tokens:

* `{{ title }}` – substituted with the route's title.
* `{{ body }}` – substituted with the route's generated or static content.

Templates are referenced by an integer ID defined in each of the `template_config.json` files, for example:

```json
{
  "2": "sample_template.html"
}
```
Will add a new template "2" to the server from the sample_template.html file

It can then be used in a json_route by setting the num_template value;
```json
{
        "function_type": "normal_page",
        "route": "/example",
        "title": "Sample Template Value",
        "body": "The body can be anything.",
        "description": "The home page",
        "template_num":2
}
```


Additional `.json` config files can be added to the templates directory.

---

## Features

* Serve static or dynamic HTML pages
* Run shell commands from the browser
* Tail log files and stream output
* Basic support for calling external APIs

---

## Installation

1. Clone this repo
2. Run `cargo build --release`
3. Configure `json_routes` and `templates` to your liking

---
