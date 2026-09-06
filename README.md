# Rusty BruhBot

<details>
<summary>The legendary <a href="https://github.com/LetUsFlow/BruhBot">BruhBot</a> impoved and rewritten in Rust.</summary>

![BruhBot Logo](logo.jpg)
</details>

## Requirements

### Pocketbase

Rusty-BruhBot uses [PocketBase](https://pocketbase.io/) to store commands an sounds and accesses the data using PocketBases REST-API.

<details>
<summary>PocketBase Schema</summary>

```json
[
  {
    "id": "zteq3osgzz3rli1",
    "listRule": "",
    "viewRule": "",
    "createRule": null,
    "updateRule": null,
    "deleteRule": null,
    "name": "sounds",
    "type": "base",
    "fields": [
      {
        "autogeneratePattern": "[a-z0-9]{15}",
        "help": "",
        "hidden": false,
        "id": "text3208210256",
        "max": 15,
        "min": 15,
        "name": "id",
        "pattern": "^[a-z0-9]+$",
        "presentable": false,
        "primaryKey": true,
        "required": true,
        "system": true,
        "type": "text"
      },
      {
        "help": "",
        "hidden": false,
        "id": "dvimlam0",
        "maxSelect": 1,
        "maxSize": 5242880,
        "mimeTypes": null,
        "name": "audio",
        "presentable": false,
        "protected": false,
        "required": true,
        "system": false,
        "thumbs": null,
        "type": "file"
      },
      {
        "autogeneratePattern": "",
        "help": "",
        "hidden": false,
        "id": "v1dwfeqt",
        "max": 0,
        "min": 0,
        "name": "command",
        "pattern": "",
        "presentable": false,
        "primaryKey": false,
        "required": true,
        "system": false,
        "type": "text"
      },
      {
        "hidden": false,
        "id": "autodate2990389176",
        "name": "created",
        "onCreate": true,
        "onUpdate": false,
        "presentable": false,
        "system": false,
        "type": "autodate"
      },
      {
        "hidden": false,
        "id": "autodate3332085495",
        "name": "updated",
        "onCreate": true,
        "onUpdate": true,
        "presentable": false,
        "system": false,
        "type": "autodate"
      }
    ],
    "indexes": [],
    "system": false
  }
]
```
</details>

### External dependencies
For this bot to work **opus** needs to be installed on your system.
For more details on how to install these dependencies, look at the [dependencies](https://github.com/serenity-rs/songbird/#dependencies) section of [Songbird](https://github.com/serenity-rs/songbird). To avoid additional dependencies, opus is the only supported audio format that BruhBot can play (but Discord uses opus for voice channels anyways).

## Configuration

Rusty-BruhBot uses environment-variables to configure the Discord-token and the PocketBase API endpoint. Alternatively, a .env-file can be used:

```bash
DISCORD_TOKEN=...
POCKETBASE_API=http://127.0.0.1:8090
```

## Deployment
It is recommended to use Docker for deployment because all dependencies except for PocketBase are bundeled with it.
The following example assumes that you have already set up and configured PocketBase as described above:
```bash
docker run -d --env-file .env --net=host ghcr.io/letusflow/rusty-bruhbot
```

## License
[GPL](LICENSE)
