import urllib.request
import concurrent.futures
import time

URL = "http://localhost:8080"
TOTAL_REQUESTS = 10000
CONCURRENCY = 200

def fetch(i):
    try:
        start = time.time()
        response = urllib.request.urlopen(URL, timeout=5)
        data = response.read()
        latency = time.time() - start
        return (True, latency)
    except Exception as e:
        return (False, 0.0)

def main():
    print(f"[STRESS TEST] Iniciando Ataque de Estresse contra {URL}")
    print(f"Alvo: {TOTAL_REQUESTS} requisicoes com {CONCURRENCY} threads simultaneas!\n")
    
    start_time = time.time()
    success = 0
    failed = 0
    total_latency = 0.0
    
    with concurrent.futures.ThreadPoolExecutor(max_workers=CONCURRENCY) as executor:
        futures = [executor.submit(fetch, i) for i in range(TOTAL_REQUESTS)]
        
        for idx, future in enumerate(concurrent.futures.as_completed(futures)):
            ok, latency = future.result()
            if ok:
                success += 1
                total_latency += latency
            else:
                failed += 1
                
            if (idx + 1) % 1000 == 0:
                print(f"[{idx + 1}/{TOTAL_REQUESTS}] Progresso...")

    end_time = time.time()
    duration = end_time - start_time
    req_per_sec = TOTAL_REQUESTS / duration if duration > 0 else 0
    avg_latency = (total_latency / success * 1000) if success > 0 else 0
    
    print("\n================================================")
    print(" RESULTADO DO TESTE DE ESTRESSE")
    print("================================================")
    print(f"Tempo Total:         {duration:.2f} segundos")
    print(f"Reqs por Segundo:    {req_per_sec:.2f} rps")
    print(f"Sucesso:             {success} ({success/TOTAL_REQUESTS*100:.1f}%)")
    print(f"Falhas:              {failed} ({failed/TOTAL_REQUESTS*100:.1f}%)")
    print(f"Latency Media:      {avg_latency:.2f} ms")
    print("================================================")

if __name__ == "__main__":
    main()
