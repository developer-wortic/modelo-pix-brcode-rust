//! Modelo da Wortic: Pix estático com QR Code, em Rust.
//!
//! Um binário só. Serve a tela (static/index.html, embutida) e estas rotas:
//!   GET /api/config                     diz só se há chave (true ou false)
//!   GET /api/gerar?chave=&nome=&cidade= monta o BR Code (com valor, txid e descricao, opcionais) e o QR
//!   GET /api/ler?codigo=                lê um copia e cola e mostra os campos
//! Sem banco, sem cobrança e sem webhook. Rode com `cargo run --release` e abra <http://127.0.0.1:8080>.
//! Variáveis: `PORTA` (padrão 8080), `ENDERECO` (padrão 127.0.0.1) e `WORTIC_API_KEY` (opcional; só liga
//! os ganchos da tela, que hoje avisam que a rota da API ainda não existe).
//!
//! Toda a lógica mora na biblioteca (`servidor` e `web`). Aqui fica só a ligação com o ambiente.

fn main() {
    std::process::exit(pix_brcode::servidor::executar(&|nome| {
        std::env::var(nome).ok()
    }));
}
