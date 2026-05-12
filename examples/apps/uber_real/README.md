# Uber Real Architecture (Matter)

Aplicativo de mobilidade urbana estruturado em camadas, feito para explorar o ecossistema Matter com arquitetura real.

## Estrutura

- `main.matter`: orquestracao da plataforma e fluxo da corrida
- `modules/domain.matter`: entidades e factories (driver, rider, ride)
- `modules/matching.matter`: estrategia de matching de motorista
- `modules/pricing.matter`: calculo de tarifa dinamica
- `modules/lifecycle.matter`: transicoes de estado da corrida
- `modules/dispatch.matter`: coordenacao de criacao de corridas
- `modules/analytics.matter`: metricas da plataforma
- `matter.toml`: manifesto local do app

## Como rodar

```bash
matter run examples/apps/uber_real/main.matter
```

## Objetivo

Esta base foi desenhada para evoluir para:

- realtime + eventos
- integracao polyglot (Python/Node/Go/Java/Rust)
- pricing adaptativo
- score de risco/seguranca
- antifraude e observabilidade
