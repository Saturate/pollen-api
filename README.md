# Pollen API Relay

Clean REST API relay for Nordic pollen data. Transforms Denmark's complex Firestore pollen API into simple, usable endpoints with multi-language support.

**Public API:** `https://pollen.akj.io`

## Quick Start

### Docker (Recommended)

```bash
docker run -d -p 3060:3060 --name pollen-api saturate/pollen-api:latest
```

Or with docker-compose:

```yaml
services:
  pollen-api:
    image: saturate/pollen-api:latest
    container_name: pollen-api
    ports:
      - "3060:3060"
    restart: unless-stopped
```

### Building from Source

```bash
cargo build --release
./target/release/pollen-api
```

API available at `http://localhost:3060`

## API Endpoints

```
GET /                                      # API info
GET /v1/dk                                # Denmark info
GET /v1/dk/regions                        # List regions
GET /v1/dk/pollen-types                   # List pollen types
GET /v1/dk/copenhagen/forecast            # Copenhagen forecast (alias: /east)
GET /v1/dk/viborg/forecast                # Viborg forecast (alias: /west)
```

### Query Parameters

- `?lang=da` - Danish translations (default: `en`)
- `?types=grass,birch` - Filter specific pollen types

### Examples

**All pollen types (English):**
```bash
curl https://pollen.akj.io/v1/dk/copenhagen/forecast
```

**Filtered types (Danish):**
```bash
curl 'https://pollen.akj.io/v1/dk/copenhagen/forecast?lang=da&types=grass,birch'
```

## Home Assistant Integration

See [Saturate/pollen](https://github.com/Saturate/pollen) for the Home Assistant HACS integration.

## License

MIT
