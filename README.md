# zedmirs

**zedmirs** is a simple mirroring service for zed extensions. 

## Features

* Downloads the latest version of all extensions for self-hosting/airgapped purposes.
* Serve extensions to Zed with the same/similar API.

## Configuration

**zedmirs** needs to be hosted behind a reverse proxy with a valid TLS certificate and dns entry to properly function. 

### Exmaple nginx config
```nginx
server {
    listen 443;
    server_name api.zed.dev;

    ssl_certificate     self-signed-registry.api.zed.dev.crt;
    ssl_certificate_key self-signed-registry.api.zed.dev.key;
    ssl_protocols       TLSv1.2 TLSv1.3;
    ssl_ciphers         HIGH:!aNULL:!MD5;
    
    index index.json;
    
    location / {
        proxy_pass http://my_zedmirs_instance:8050;

    }
} 
```

## Commands

zedmirs operations are run via the command line and has two mode of operations; **mirror** and **serve**:


* `mirror`: Download metadata and extensions from the official source. Creates an index to be used when running `serve`.
* `serve`: Serves extensions using the same API as the official sources. `mirror` needs to have been run first to populate the output path with extensions and the index.

### Command options

| Long option    | Short option | ENV variable  | Description |
| ---------------| ------------ | ------------- | ----------- |
| --dl-threads   | -d           | DL_THREADS=   | The maximum number of concurrent mirror download tasks. *Works only with the `mirror` commands*. [default: 8] |
| --output       | -o           | OUTPUT=       | The directory into where the mirrors will be downloaded. |
| --help         | -h           |               | Print help. |
| --version      | -V           |               | Print version. |

### Command examples

Mirror operation
```
./zedmirs --output /opt/mirror-root mirror
```

Serve operation
```
./aptmirs --output /opt/mirror-root serve
```
