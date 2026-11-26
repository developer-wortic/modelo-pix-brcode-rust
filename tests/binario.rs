//! Roda o binário de verdade e fala com ele por HTTP, sem crate extra: um `TcpStream` basta.
//! Tudo fica na máquina local (127.0.0.1 ou `::1`), sem rede externa.

use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// Contador da suíte: cada porta pedida é diferente da anterior, para dois testes nunca dividirem uma porta.
static PEDIDOS: AtomicUsize = AtomicUsize::new(0);

/// Uma porta que nenhum outro teste desta suíte recebeu. Fica abaixo da faixa efêmera do sistema
/// (a partir de 49152), para não colidir com conexões de saída de outros programas.
fn porta_livre() -> u16 {
    let n = PEDIDOS.fetch_add(1, Ordering::SeqCst);
    20_000 + u16::try_from((std::process::id() as usize * 7 + n * 13) % 20_000).unwrap()
}

/// Um binário no ar, derrubado quando sai do escopo.
struct Servidor {
    filho: Child,
    porta: u16,
}

impl Drop for Servidor {
    fn drop(&mut self) {
        let _ = self.filho.kill();
        let _ = self.filho.wait();
    }
}

fn subir(ambiente: &[(&str, &str)]) -> Servidor {
    let porta = porta_livre();
    let mut comando = Command::new(env!("CARGO_BIN_EXE_pix-brcode"));
    comando
        .env_remove("WORTIC_API_KEY")
        .env("PORTA", porta.to_string())
        .env("ENDERECO", "127.0.0.1")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (chave, valor) in ambiente {
        comando.env(chave, valor);
    }
    let filho = comando.spawn().expect("sobe o binário");
    let servidor = Servidor { filho, porta };
    esperar(&format!("127.0.0.1:{porta}"));
    servidor
}

/// Espera o binário responder HTTP (até 10 segundos). Uma conexão aberta não basta:
/// tem de vir a resposta do nosso servidor.
fn esperar(endereco: &str) {
    let fim = Instant::now() + Duration::from_secs(10);
    while Instant::now() < fim {
        if responde(endereco) {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("o binário não respondeu em {endereco}");
}

/// Um GET de teste: `true` se a resposta começa com 200.
fn responde(endereco: &str) -> bool {
    let Ok(mut s) = std::net::TcpStream::connect(endereco) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = write!(
        s,
        "GET /api/config HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    );
    let mut bruto = String::new();
    let _ = s.read_to_string(&mut bruto);
    bruto.starts_with("HTTP/1.1 200")
}

/// Uma resposta HTTP: status e corpo.
struct Visto {
    status: u16,
    corpo: String,
}

fn pedir(servidor: &Servidor, caminho: &str) -> Visto {
    pedir_em(&format!("127.0.0.1:{}", servidor.porta), caminho)
}

fn pedir_em(endereco: &str, caminho: &str) -> Visto {
    let mut s = std::net::TcpStream::connect(endereco).expect("conecta");
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    write!(
        s,
        "GET {caminho} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut bruto = String::new();
    s.read_to_string(&mut bruto).unwrap();
    let status = bruto
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("linha de status");
    let corpo = bruto
        .split_once("\r\n\r\n")
        .map_or_else(String::new, |(_, c)| c.to_string());
    Visto { status, corpo }
}

/// Codifica para a query: o que não é letra, número ou `-._~` vira %XX.
fn codificar(texto: &str) -> String {
    texto
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[test]
fn serve_a_tela_e_a_config_sem_mostrar_a_chave() {
    let s = subir(&[("WORTIC_API_KEY", "segredo-de-teste")]);
    let tela = pedir(&s, "/");
    assert_eq!(tela.status, 200);
    assert!(tela.corpo.contains("Pix com QR Code"));
    let config = pedir(&s, "/api/config");
    assert_eq!(config.status, 200);
    assert_eq!(config.corpo, "{\"chave\":true}");
    assert!(!config.corpo.contains("segredo-de-teste"));
}

#[test]
fn sem_chave_a_config_diz_false() {
    let s = subir(&[("WORTIC_API_KEY", "")]);
    assert_eq!(pedir(&s, "/api/config").corpo, "{\"chave\":false}");
}

#[test]
fn gerar_e_ler_pelo_http_com_a_mesma_cobranca() {
    let s = subir(&[]);
    let consulta = format!(
        "/api/gerar?chave={}&nome={}&cidade={}&valor={}",
        codificar("123.456.789-09"),
        codificar("Loja Exemplo"),
        codificar("Sao Paulo"),
        codificar("10,50")
    );
    let gerado = pedir(&s, &consulta);
    assert_eq!(gerado.status, 200, "{}", gerado.corpo);
    assert!(gerado.corpo.contains("\"tipo_chave\":\"CPF\""));
    assert!(gerado.corpo.contains("<svg "));
    let codigo = gerado
        .corpo
        .split("\"codigo\":\"")
        .nth(1)
        .and_then(|r| r.split('"').next())
        .expect("código no JSON")
        .to_string();
    assert!(codigo.starts_with("00020101021126"));
    assert!(codigo.contains("540510.50"));

    let lido = pedir(&s, &format!("/api/ler?codigo={}", codificar(&codigo)));
    assert_eq!(lido.status, 200, "{}", lido.corpo);
    assert!(lido.corpo.contains("\"chave\":\"12345678909\""));
    assert!(lido.corpo.contains("\"valor\":\"10.50\""));
    assert!(lido.corpo.contains("\"nome\":\"Loja Exemplo\""));
}

#[test]
fn gerar_sem_campo_obrigatorio_da_400() {
    let s = subir(&[]);
    let r = pedir(&s, "/api/gerar?nome=Ana&cidade=Recife");
    assert_eq!(r.status, 400);
    assert!(r.corpo.contains("chave Pix é obrigatório."));
}

#[test]
fn ler_codigo_alterado_da_400_com_o_motivo() {
    let s = subir(&[]);
    let r = pedir(
        &s,
        "/api/ler?codigo=00020101021126330014br.gov.bcb.pix0111123456789095204000053039865802BR5912Loja%20Exemplo6009Sao%20Paulo62070503***630400AA",
    );
    assert_eq!(r.status, 400);
    assert!(r.corpo.contains("CRC não confere"));
}

#[test]
fn rota_que_nao_existe_da_404_em_json() {
    let s = subir(&[]);
    let r = pedir(&s, "/nao_existe");
    assert_eq!(r.status, 404);
    assert!(r.corpo.contains("Rota não encontrada."));
}

#[test]
fn porta_que_nao_abre_sai_com_a_mensagem() {
    let saida = Command::new(env!("CARGO_BIN_EXE_pix-brcode"))
        .env("PORTA", "abc")
        .env("ENDERECO", "127.0.0.1")
        .output()
        .expect("roda o binário");
    assert!(!saida.status.success());
    let erro = String::from_utf8_lossy(&saida.stderr);
    assert!(
        erro.contains("não consegui abrir a porta"),
        "stderr: {erro}"
    );
}

#[test]
fn endereco_ipv6_sobe_entre_colchetes() {
    let porta = porta_livre();
    let mut comando = Command::new(env!("CARGO_BIN_EXE_pix-brcode"));
    comando
        .env("PORTA", porta.to_string())
        .env("ENDERECO", "::1")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut filho = comando.spawn().expect("sobe o binário");
    let endereco = format!("[::1]:{porta}");
    let fim = Instant::now() + Duration::from_secs(10);
    let subiu = loop {
        if std::net::TcpStream::connect(&endereco).is_ok() {
            break true;
        }
        if Instant::now() > fim {
            break false;
        }
        thread::sleep(Duration::from_millis(20));
    };
    if subiu {
        let r = pedir_em(&endereco, "/api/config");
        assert_eq!(r.status, 200);
    }
    let _ = filho.kill();
    let _ = filho.wait();
    assert!(subiu, "não abriu [::1]:{porta}");
}
