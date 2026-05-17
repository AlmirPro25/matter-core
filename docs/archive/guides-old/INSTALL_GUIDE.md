# Guia de Instalação do Matter

## 🎯 Como Instalar Matter no Windows

### Método 1: Instalação Automática (Recomendado)

1. **Abra o PowerShell como Administrador**
   - Clique com botão direito no Menu Iniciar
   - Escolha "Windows PowerShell (Admin)" ou "Terminal (Admin)"

2. **Navegue até a pasta do Matter**
   ```powershell
   cd "F:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE"
   ```

3. **Execute o instalador**
   ```powershell
   .\install.ps1
   ```

4. **Aguarde a instalação**
   - O sistema vai compilar o Matter
   - Copiar arquivos para `C:\Program Files\Matter`
   - Adicionar ao PATH do Windows
   - Criar atalhos no Menu Iniciar

5. **Feche e abra um novo terminal**
   - Isso é necessário para o PATH funcionar

6. **Teste a instalação**
   ```powershell
   matter --help
   ```

### Método 2: Instalação Manual

Se preferir fazer manualmente:

1. **Compile o projeto**
   ```powershell
   cargo build --release
   ```

2. **Crie a pasta de instalação**
   ```powershell
   mkdir "C:\Program Files\Matter\bin"
   ```

3. **Copie o executável**
   ```powershell
   copy target\release\matter-cli.exe "C:\Program Files\Matter\bin\matter.exe"
   ```

4. **Adicione ao PATH**
   - Abra "Variáveis de Ambiente" no Windows
   - Edite a variável "Path" do Sistema
   - Adicione: `C:\Program Files\Matter\bin`

5. **Reinicie o terminal**

---

## 🚀 Usando o Matter Após Instalação

### Comandos Básicos

```powershell
# Ver ajuda
matter --help

# Executar um arquivo
matter run meu_programa.matter

# Compilar para bytecode
matter compile meu_programa.matter -o programa.mbc

# Executar bytecode
matter run-bytecode programa.mbc

# Ver exemplos
cd "C:\Program Files\Matter\examples"
matter run hello.matter
```

### Criar Seu Primeiro Programa

1. **Crie um arquivo `hello.matter`**
   ```matter
   let nome = "Almir"
   print "Olá, " + nome + "!"
   
   fn somar(a, b) {
       return a + b
   }
   
   let resultado = somar(10, 20)
   print resultado
   ```

2. **Execute**
   ```powershell
   matter run hello.matter
   ```

---

## 📦 O Que Foi Instalado?

### Estrutura de Arquivos

```
C:\Program Files\Matter\
├── bin\
│   └── matter.exe          # Executável principal
├── examples\               # Exemplos de código
│   ├── hello.matter
│   ├── functions.matter
│   └── ...
├── docs\                   # Documentação
│   ├── MANIFESTO.md
│   ├── SPEC.md
│   └── ...
└── README.md              # Guia rápido
```

### PATH do Sistema

O instalador adiciona `C:\Program Files\Matter\bin` ao PATH, permitindo usar `matter` de qualquer lugar.

### Atalhos

- **Menu Iniciar** → Matter → Matter REPL (quando implementado)

---

## 🔧 Desinstalação

### Método Automático

1. **Abra PowerShell como Administrador**

2. **Navegue até a pasta do Matter**
   ```powershell
   cd "F:\Users\almir\Desktop\MANIFESTO DA LINGUAGEM MATTER CORE"
   ```

3. **Execute o desinstalador**
   ```powershell
   .\uninstall.ps1
   ```

### Método Manual

1. **Remova do PATH**
   - Variáveis de Ambiente → Path → Remover `C:\Program Files\Matter\bin`

2. **Delete a pasta**
   ```powershell
   Remove-Item "C:\Program Files\Matter" -Recurse -Force
   ```

---

## 🐛 Solução de Problemas

### "matter não é reconhecido como comando"

**Causa**: PATH não foi atualizado ou terminal não foi reiniciado

**Solução**:
1. Feche TODOS os terminais abertos
2. Abra um novo terminal
3. Teste novamente

### "Acesso negado" durante instalação

**Causa**: PowerShell não está rodando como Administrador

**Solução**:
1. Feche o PowerShell
2. Clique com botão direito no Menu Iniciar
3. Escolha "Windows PowerShell (Admin)"
4. Execute o instalador novamente

### Erro de compilação

**Causa**: Rust/Cargo não está instalado

**Solução**:
1. Instale Rust: https://rustup.rs/
2. Reinicie o terminal
3. Execute o instalador novamente

---

## 📚 Próximos Passos

Após instalar:

1. **Explore os exemplos**
   ```powershell
   cd "C:\Program Files\Matter\examples"
   dir
   matter run hello.matter
   ```

2. **Leia a documentação**
   ```powershell
   cd "C:\Program Files\Matter\docs"
   notepad SPEC.md
   ```

3. **Crie seus próprios programas**
   ```powershell
   mkdir C:\MeusProgramasMatter
   cd C:\MeusProgramasMatter
   notepad meu_programa.matter
   matter run meu_programa.matter
   ```

---

## 🎉 Pronto!

Agora você pode usar Matter igual ao Node.js:
- ✅ Instalado no sistema
- ✅ Disponível em qualquer pasta
- ✅ Comando `matter` funcionando
- ✅ Exemplos e documentação inclusos

**Divirta-se programando em Matter!** 🚀
