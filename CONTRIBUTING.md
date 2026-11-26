# Como contribuir

Obrigado por querer ajudar. Este é um modelo pequeno, feito para ser fácil de ler e de adaptar.

## Antes de começar

- Para mudança grande, abra uma issue antes do código.
- Não ponha chave Pix, CPF, CNPJ, telefone ou e-mail real em issue ou pull request. Use só dados de exemplo, como `contato@exemplo.com.br`, `LOJA EXEMPLO` e `SAO PAULO`.
- Se a mudança toca a montagem ou a leitura do BR Code, diga qual parte do manual do Banco Central ela segue, com o número da seção ou da versão.

## Rodar as checagens

```sh
cargo fmt --check
cargo clippy -- -D warnings -W clippy::pedantic
cargo test
```

O mesmo roda no GitHub Actions em cada push e em cada pull request.

Para medir a cobertura de linhas, use `cargo llvm-cov`. A meta deste modelo é 100% das linhas.

## Regras do projeto

- Dependência nova precisa de motivo claro. A biblioteca não depende de rede nem de C.
- Mudança de código vem com teste.
- A tela continua em HTML puro, em `static/index.html`, sem build de front-end.
- O projeto usa a licença AAL (uso livre com créditos à Wortic). Ao contribuir, você concorda que sua contribuição entra sob a mesma licença.

## Pull request

- Diga o que muda e por quê.
- Inclua a saída de `cargo test`.
