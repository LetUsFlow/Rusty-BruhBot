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
        "name": "sounds",
        "type": "base",
        "system": false,
        "schema": [
            {
                "id": "dvimlam0",
                "name": "audio",
                "type": "file",
                "system": false,
                "required": true,
                "unique": false,
                "options": {
                    "maxSelect": 1,
                    "maxSize": 5242880,
                    "mimeTypes": [],
                    "thumbs": []
                }
            },
            {
                "id": "v1dwfeqt",
                "name": "command",
                "type": "text",
                "system": false,
                "required": true,
                "unique": true,
                "options": {
                    "min": null,
                    "max": null,
                    "pattern": ""
                }
            }
        ],
        "listRule": "",
        "viewRule": "",
        "createRule": null,
        "updateRule": null,
        "deleteRule": null,
        "options": {}
    }
]
```
</details>

### External dependencies
For this bot to work, **opus** and **ffmpeg** need to be installed on your system.
For more details on how to install these dependencies, look at the [dependencies](https://github.com/serenity-rs/songbird/#dependencies) section of [Songbird](https://github.com/serenity-rs/songbird) (Note: youtube-dl is not required).

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
[ISC](LICENSE)
