
# Local Rust Web Server

A lightweight, minimal-profile web server designed for use on a local network (e.g., a Raspberry Pi). It provides access to route-configured web endpoints that can return HTML pages or execute system commands.

This server uses a combination of JSON and HTML files to define dynamic routes and serve formatted HTML content.

Very much a work in progress.

---

## Overview

* HTML templates are stored in the `/templates` directory and are associated with integer keys defined in `template_config.json`.
* Each API route is defined by a JSON file in the `/json_routes` directory.
* Routes are mapped at startup and exposed as web endpoints.
* `/help` will display
---

## Route Configuration

Each JSON route file can define one or many route objects with the following fields:

| Field           | Description                                                                                     |
| --------------- | ----------------------------------------------------------------------------------------------- |
| `function_type` | The behavior type of the route. Supported types: `"normal_page"`, `"run_command"`, `"get_logs"` |
| `title`         | The display title of the page.                                                                  |
| `description`   | A short help description of the endpoint, meant for use on the `/help` page.                    |
| `template_num`  | The template number to use. Defaults to `0` if not specified.                                   |

---

## Templates

Each template is a standalone HTML file that uses substitution tokens:

* `{{ title }}` – substituted with the route's title.
* `{{ body }}` – substituted with the route's generated or static content.

Templates are referenced by their integer ID in `template_config.json`, for example:

```json
{
  "0": "default_template.html",
  "1": "logs_template.html"
}
```

---

## Features

*  Serve static or dynamic HTML pages
*  Run shell commands from the browser
*  Tail log files and stream output
*  Basic frontend support for proxying API data

---

## Example Use Case

Access the server at `http://raspberrypi.local/`, click into specific routes defined via JSON, and:

* View system status pages
* Run shell scripts via a simple interface
* Check log files directly from the browser

---
