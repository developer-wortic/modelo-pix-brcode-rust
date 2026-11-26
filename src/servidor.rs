//! O servidor HTTP: sobe a porta, atende os pedidos com algumas threads e liga cada pedido às rotas de `web`.
//! Mora na biblioteca para que os testes rodem no próprio processo. O binário só chama `principal`.

use std::convert::Infallible;
use std::sync::Arc;
use std::thread;

use tiny_http::{Header, Response, Server};

use crate::web;

/// Quantas threads atendem os pedidos.
pub const ATENDENTES: usize = 4;

/// O que o binário chama. Sobe o servidor e fica atendendo até o processo parar.
/// Só volta se a porta não abrir: aí imprime a mensagem e devolve 1 como código de saída.
pub fn executar(ambiente: &dyn Fn(&str) -> Option<String>) -> i32 {
    let Err(erro) = principal(ambiente);
    eprintln!("{erro}");
    1
}

/// Lê a configuração do ambiente, abre a porta e atende para sempre.
/// Nunca devolve `Ok`: o único retorno possível é o erro de abrir a porta.
///
/// # Errors
///
/// Devolve a mensagem em português quando não consegue abrir a porta.
pub fn principal(ambiente: &dyn Fn(&str) -> Option<String>) -> Result<Infallible, String> {
    let endereco = endereco_de(ambiente("ENDERECO"));
    let porta = porta_de(ambiente("PORTA"));
    let tem_chave = tem_chave_de(ambiente("WORTIC_API_KEY"));
    let servidor = subir(&endereco, &porta)?;
    println!(
        "Pix com QR Code no ar em http://{}:{porta}",
        colchetes(&endereco)
    );
    servir(&servidor, tem_chave)
}

/// A porta vem de `PORTA`; sem ela, 8080.
#[must_use]
pub fn porta_de(valor: Option<String>) -> String {
    valor.unwrap_or_else(|| "8080".to_string())
}

/// O endereço vem de `ENDERECO`; sem ele, 127.0.0.1 (só a própria máquina).
#[must_use]
pub fn endereco_de(valor: Option<String>) -> String {
    valor
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

/// Com chave só quer dizer que a variável existe e não está vazia. O valor nunca sai.
#[must_use]
pub fn tem_chave_de(valor: Option<String>) -> bool {
    valor.is_some_and(|v| !v.is_empty())
}

/// Endereço IPv6 vai entre colchetes, como `[::1]:8080`. Os outros ficam como estão.
#[must_use]
pub fn colchetes(endereco: &str) -> String {
    if endereco.contains(':') {
        format!("[{endereco}]")
    } else {
        endereco.to_string()
    }
}

/// Abre a porta no endereço. Se não abrir, a mensagem diz qual porta e qual endereço.
///
/// # Errors
///
/// Devolve a mensagem em português quando o `tiny_http` não consegue abrir a porta.
pub fn subir(endereco: &str, porta: &str) -> Result<Arc<Server>, String> {
    Server::http(format!("{}:{porta}", colchetes(endereco)))
        .map(Arc::new)
        .map_err(|erro| format!("não consegui abrir a porta {porta} em {endereco}: {erro}"))
}

/// Atende os pedidos com `ATENDENTES` threads, para sempre. A thread principal fica parada,
/// e quem encerra o processo é o sistema operacional (Ctrl+C, por exemplo).
pub fn servir(servidor: &Arc<Server>, tem_chave: bool) -> ! {
    for _ in 0..ATENDENTES {
        let servidor = servidor.clone();
        thread::spawn(move || {
            for pedido in servidor.incoming_requests() {
                atender(pedido, tem_chave);
            }
        });
    }
    loop {
        thread::park();
    }
}

fn cabecalho(nome: &str, valor: &str) -> Header {
    Header::from_bytes(nome.as_bytes(), valor.as_bytes()).expect("cabeçalho válido")
}

/// Um pedido vira uma resposta de `web::rota`, com o tipo do conteúdo.
pub fn atender(pedido: tiny_http::Request, tem_chave: bool) {
    let url = pedido.url().to_string();
    let (caminho, consulta) = url.split_once('?').unwrap_or((url.as_str(), ""));
    let r = web::rota(caminho, consulta, tem_chave);
    let resposta = Response::from_string(r.corpo)
        .with_status_code(r.status)
        .with_header(cabecalho("content-type", r.tipo));
    let _ = pedido.respond(resposta);
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    /// Contador dos testes: cada porta pedida é diferente, e só vale a porta que o `bind` confirma livre.
    static PEDIDOS: AtomicUsize = AtomicUsize::new(0);

    fn porta_livre() -> u16 {
        let n = PEDIDOS.fetch_add(1, Ordering::SeqCst);
        20_000 + u16::try_from((std::process::id() as usize * 5 + n * 11) % 20_000).unwrap()
    }

    /// GET em texto cru, para ler o status e o corpo sem crate extra.
    fn pedir(porta: u16, caminho: &str) -> (u16, String) {
        let mut s = TcpStream::connect(("127.0.0.1", porta)).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        write!(
            s,
            "GET {caminho} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut bruto = String::new();
        s.read_to_string(&mut bruto).unwrap();
        let status = bruto.split_whitespace().nth(1).unwrap().parse().unwrap();
        let corpo = bruto
            .split_once("\r\n\r\n")
            .map_or_else(String::new, |(_, c)| c.to_string());
        (status, corpo)
    }

    /// Espera a porta abrir (até 10 segundos) e confirma com uma conexão no fim.
    /// A pausa vem antes de cada tentativa, então o laço sempre roda ao menos uma vez.
    fn espera_abrir(porta: u16) {
        let fim = Instant::now() + Duration::from_secs(10);
        let mut aberta = false;
        while !aberta && Instant::now() < fim {
            thread::sleep(Duration::from_millis(20));
            aberta = TcpStream::connect(("127.0.0.1", porta)).is_ok();
        }
        assert!(aberta, "a porta {porta} não abriu");
    }

    #[test]
    fn porta_padrao_e_a_pedida() {
        assert_eq!(porta_de(None), "8080");
        assert_eq!(porta_de(Some("9000".into())), "9000");
    }

    #[test]
    fn endereco_padrao_e_o_local() {
        assert_eq!(endereco_de(None), "127.0.0.1");
        assert_eq!(endereco_de(Some(String::new())), "127.0.0.1");
        assert_eq!(endereco_de(Some("0.0.0.0".into())), "0.0.0.0");
    }

    #[test]
    fn chave_so_conta_se_nao_estiver_vazia() {
        assert!(!tem_chave_de(None));
        assert!(!tem_chave_de(Some(String::new())));
        assert!(tem_chave_de(Some("x".into())));
    }

    #[test]
    fn ipv6_vai_entre_colchetes_e_ipv4_nao() {
        assert_eq!(colchetes("::1"), "[::1]");
        assert_eq!(colchetes("127.0.0.1"), "127.0.0.1");
    }

    #[test]
    fn porta_ocupada_da_mensagem_com_porta_e_endereco() {
        let ocupada = TcpListener::bind("127.0.0.1:0").unwrap();
        let porta = ocupada.local_addr().unwrap().port().to_string();
        // se a porta não estivesse ocupada, o erro sairia vazio e a asserção abaixo falharia
        let erro = subir("127.0.0.1", &porta).err().unwrap_or_default();
        assert!(erro.starts_with("não consegui abrir a porta"), "{erro}");
        assert!(erro.contains(&porta));
    }

    #[test]
    fn principal_sobe_e_atende_pelo_ambiente() {
        let porta = porta_livre();
        let porta_txt = porta.to_string();
        thread::spawn(move || {
            let _ = principal(&|nome| match nome {
                "PORTA" => Some(porta_txt.clone()),
                "WORTIC_API_KEY" => Some("chave-de-teste".to_string()),
                _ => None,
            });
        });
        espera_abrir(porta);
        let (status, corpo) = pedir(porta, "/api/config");
        assert_eq!(status, 200);
        assert_eq!(corpo, "{\"chave\":true}");
        let (status, _) = pedir(porta, "/");
        assert_eq!(status, 200);
    }

    #[test]
    fn executar_com_porta_ruim_devolve_codigo_1() {
        let codigo = executar(&|nome| (nome == "PORTA").then(|| "abc".to_string()));
        assert_eq!(codigo, 1);
    }

    #[test]
    fn principal_com_porta_ruim_devolve_o_erro() {
        let erro = principal(&|nome| (nome == "PORTA").then(|| "abc".to_string())).err();
        assert!(
            erro.unwrap_or_default()
                .contains("não consegui abrir a porta abc")
        );
    }

    #[test]
    fn atender_responde_404_e_conteudo_json() {
        let porta = porta_livre();
        let servidor = subir("127.0.0.1", &porta.to_string()).unwrap();
        let no_fio = servidor.clone();
        thread::spawn(move || servir(&no_fio, false));
        espera_abrir(porta);
        let (status, corpo) = pedir(porta, "/rota_que_nao_existe");
        assert_eq!(status, 404);
        assert!(corpo.contains("Rota não encontrada."));
        let (status, _) = pedir(porta, "/api/gerar?nome=Ana&cidade=Recife");
        assert_eq!(status, 400);
    }
}
