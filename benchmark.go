package main

import (
	"fmt"
)

func fib(n int) int {
	if n < 2 {
		return n
	}
	return fib(n-1) + fib(n-2)
}

func main() {
	fmt.Println("Iniciando Teste de Furia (Go)...")
	
	fmt.Println("1. Fibonacci(35)...")
	startFib := fib(35)
	fmt.Println("Resultado:", startFib)

	fmt.Println("2. Loop pesado com manipulacao de Dicionario/Map na Heap")
	d := make(map[string]int)
	d["contador"] = 0
	
	i := 0
	for i < 10000 {
		d["contador"] = d["contador"] + 1
		i = i + 1
	}

	fmt.Println("Contador processado:", d["contador"])
}
