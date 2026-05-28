# jcode Configuration Repository

Este directorio contiene la configuración compartida de jcode para este proyecto.

## Contenido

- `swarm-optimized.toml` - Configuración optimizada para 5 agents swarm
- `skills/` - Skills personalizados
- `memory_projects/` - Memorias de proyecto
- `latency_benchmark.jsonl` - Datos de benchmark

## No incluir (sensitive)

No commitea estos archivos:
- `config.toml` (contiene API keys)
- `auth*.json` (credenciales)
- `*.env` (secrets)
- `memory/projects/` (datos sensibles de proyecto)
- `sessions/` (conversaciones privadas)
- `telemetry_*.txt` (IDs únicos)
