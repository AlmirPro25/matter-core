# Uber Real Prod (Matter)

Arquitetura de referencia para um app estilo Uber com separacao de camadas e pontos de extensao para todo o ecossistema Matter.

## Camadas

- `modules/api.matter`: entrada e normalizacao de requests
- `modules/security.matter`: gate de risco e validacao
- `modules/matching.matter`: selecao de motorista
- `modules/pricing.matter`: surge + tarifa
- `modules/dispatch.matter`: construcao de ride/response
- `modules/lifecycle.matter`: estados da corrida
- `modules/realtime.matter`: eventos de dominio
- `modules/analytics.matter`: metricas operacionais
- `modules/forecast_polyglot.matter`: hook para ML/polyglot
- `contracts/api_contract.matter`: contratos de entrada/saida

## Execucao

```bash
matter run examples/apps/uber_real_prod/main.matter
```

Simulador avancado (single-file, ideal para validação):

```bash
matter run examples/apps/uber_real_prod/simulator.matter
```

Orquestrador operacional (fila + SLA + rebalanco):

```bash
matter run examples/apps/uber_real_prod/orchestrator_sim.matter
```

## Proximos upgrades

1. Trocar `demand_forecast_local_fallback` por modelo Python real
2. Ligar `realtime` em websocket
3. Persistir rides/eventos em DB
4. Expor `api` via `web_api` do ecossistema
