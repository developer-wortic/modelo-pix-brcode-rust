//! As rotas do servidor, sem sockets: cada pedido vira uma `Resposta`. Assim a lógica
//! é testável sem subir a porta. O `servidor` só liga isso ao `tiny_http`.

use std::fmt::Write;

use crate::cobranca::Cobranca;
use crate::erro::Erro;
use crate::{leitura, qr};

/// A tela, embutida no binário.
pub const TELA: &str = include_str!("../static/index.html");

/// Status, tipo do conteúdo e corpo de uma resposta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resposta {
    pub status: u16,
    pub tipo: &'static str,
    pub corpo: String,
}

const JSON: &str = "application/json; charset=utf-8";
const HTML: &str = "text/html; charset=utf-8";

/// Decide a resposta de um caminho. `consulta` é o que vem depois do `?`, sem o `?`.
/// `tem_chave` diz só se `WORTIC_API_KEY` está definida: a chave em si nunca sai daqui.
#[must_use]
pub fn rota(caminho: &str, consulta: &str, tem_chave: bool) -> Resposta {
    match caminho {
        "/" => Resposta {
            status: 200,
            tipo: HTML,
            corpo: TELA.to_string(),
        },
        "/api/config" => json(200, format!("{{\"chave\":{tem_chave}}}")),
        "/api/gerar" => resposta_de(gerar(&parametros(consulta))),
        "/api/ler" => resposta_de(ler_codigo(&parametros(consulta))),
        _ => json(404, erro_json("Rota não encontrada.")),
    }
}

fn json(status: u16, corpo: String) -> Resposta {
    Resposta {
        status,
        tipo: JSON,
        corpo,
    }
}

fn resposta_de(resultado: Result<String, Erro>) -> Resposta {
    match resultado {
        Ok(corpo) => json(200, corpo),
        Err(erro) => json(400, erro_json(&erro.to_string())),
    }
}

fn erro_json(mensagem: &str) -> String {
    format!("{{\"erro\":{{\"mensagem\":{}}}}}", texto_json(mensagem))
}

/// Monta a cobrança estática: devolve o copia e cola, o QR em SVG e o tipo da chave.
fn gerar(p: &[(String, String)]) -> Result<String, Erro> {
    let mut cobranca = Cobranca::nova(
        obrigatorio(p, "chave", "chave Pix")?,
        obrigatorio(p, "nome", "nome do recebedor")?,
        obrigatorio(p, "cidade", "cidade")?,
    )?;
    if let Some(valor) = opcional(p, "valor") {
        cobranca = cobranca.com_valor(valor)?;
    }
    if let Some(txid) = opcional(p, "txid") {
        cobranca = cobranca.com_txid(txid)?;
    }
    if let Some(descricao) = opcional(p, "descricao") {
        cobranca = cobranca.com_descricao(descricao)?;
    }
    let codigo = cobranca.codigo()?;
    let svg = qr::svg(&codigo)?;
    Ok(format!(
        "{{\"codigo\":{},\"qr_svg\":{},\"tipo_chave\":{}}}",
        texto_json(&codigo),
        texto_json(&svg),
        texto_json(cobranca.chave.tipo().nome())
    ))
}

/// Lê um copia e cola e devolve os campos.
fn ler_codigo(p: &[(String, String)]) -> Result<String, Erro> {
    let l = leitura::ler(obrigatorio(p, "codigo", "código Pix")?)?;
    let campos: Vec<String> = l
        .campos
        .iter()
        .map(|c| {
            format!(
                "{{\"id\":{},\"valor\":{}}}",
                texto_json(&c.id),
                texto_json(&c.valor)
            )
        })
        .collect();
    Ok(format!(
        "{{\"chave\":{},\"descricao\":{},\"valor\":{},\"moeda\":{},\"pais\":{},\"nome\":{},\"cidade\":{},\"categoria\":{},\"txid\":{},\"campos\":[{}]}}",
        texto_json(&l.chave),
        opcional_json(l.descricao.as_deref()),
        opcional_json(l.valor.as_deref()),
        texto_json(&l.moeda),
        texto_json(&l.pais),
        texto_json(&l.nome),
        texto_json(&l.cidade),
        texto_json(&l.categoria),
        opcional_json(l.txid.as_deref()),
        campos.join(",")
    ))
}

fn obrigatorio<'a>(
    p: &'a [(String, String)],
    nome: &str,
    campo: &'static str,
) -> Result<&'a str, Erro> {
    opcional(p, nome).ok_or(Erro::Obrigatorio(campo))
}

/// O valor de um parâmetro, sem espaços nas pontas. Vazio conta como ausente.
fn opcional<'a>(p: &'a [(String, String)], nome: &str) -> Option<&'a str> {
    p.iter()
        .find(|(k, _)| k == nome)
        .map(|(_, v)| v.trim())
        .filter(|v| !v.is_empty())
}

/// Os pares `chave=valor` da consulta, já decodificados.
#[must_use]
pub fn parametros(consulta: &str) -> Vec<(String, String)> {
    consulta
        .split('&')
        .filter(|par| !par.is_empty())
        .map(|par| {
            let (k, v) = par.split_once('=').unwrap_or((par, ""));
            (decodificar(k), decodificar(v))
        })
        .collect()
}

/// Desfaz o `+` (espaço) e os `%XX` da URL. `%` sem dois dígitos hexadecimais passa como veio.
#[must_use]
pub fn decodificar(texto: &str) -> String {
    let b = texto.as_bytes();
    let mut saida = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'+' {
            saida.push(b' ');
            i += 1;
        } else if b[i] == b'%'
            && i + 2 < b.len()
            && b[i + 1].is_ascii_hexdigit()
            && b[i + 2].is_ascii_hexdigit()
        {
            let hex = std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("0");
            saida.push(u8::from_str_radix(hex, 16).unwrap_or(b'%'));
            i += 3;
        } else {
            saida.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&saida).into_owned()
}

fn opcional_json(valor: Option<&str>) -> String {
    valor.map_or_else(|| "null".to_string(), texto_json)
}

/// Põe um texto entre aspas no formato JSON, escapando o que precisa.
#[must_use]
pub fn texto_json(texto: &str) -> String {
    let mut saida = String::with_capacity(texto.len() + 2);
    saida.push('"');
    for c in texto.chars() {
        match c {
            '"' => saida.push_str("\\\""),
            '\\' => saida.push_str("\\\\"),
            '\n' => saida.push_str("\\n"),
            '\r' => saida.push_str("\\r"),
            '\t' => saida.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(saida, "\\u{:04x}", u32::from(c));
            }
            c => saida.push(c),
        }
    }
    saida.push('"');
    saida
}

#[cfg(test)]
mod testes {
    use super::*;

    const CHAVE: &str = "chave=123.456.789-09&nome=Loja+Exemplo&cidade=Sao+Paulo";

    fn corpo(r: &Resposta) -> &str {
        &r.corpo
    }

    #[test]
    fn a_raiz_entrega_a_tela() {
        let r = rota("/", "", false);
        assert_eq!(r.status, 200);
        assert_eq!(r.tipo, HTML);
        assert!(corpo(&r).contains("<title>"));
    }

    #[test]
    fn config_diz_so_se_ha_chave() {
        assert_eq!(rota("/api/config", "", true).corpo, "{\"chave\":true}");
        assert_eq!(rota("/api/config", "", false).corpo, "{\"chave\":false}");
    }

    #[test]
    fn gerar_devolve_codigo_qr_e_tipo() {
        let r = rota("/api/gerar", &format!("{CHAVE}&valor=10%2C50"), false);
        assert_eq!(r.status, 200);
        assert_eq!(r.tipo, JSON);
        assert!(corpo(&r).starts_with("{\"codigo\":\"000201"));
        assert!(corpo(&r).contains("\"tipo_chave\":\"CPF\""));
        assert!(corpo(&r).contains("\"qr_svg\":\"<svg "));
        assert!(corpo(&r).contains("540510.50"));
    }

    #[test]
    fn gerar_com_campo_faltando_da_400_com_a_mensagem() {
        let r = rota("/api/gerar", "nome=Ana&cidade=Recife", false);
        assert_eq!(r.status, 400);
        assert!(corpo(&r).contains("chave Pix é obrigatório."));
        let r = rota(
            "/api/gerar",
            "chave=123.456.789-09&nome=&cidade=Recife",
            false,
        );
        assert!(corpo(&r).contains("nome do recebedor é obrigatório."));
        let r = rota("/api/gerar", "chave=123.456.789-09&nome=Ana", false);
        assert!(corpo(&r).contains("cidade é obrigatório."));
    }

    #[test]
    fn gerar_com_valor_txid_descricao_e_erro_de_cada_um() {
        let base = CHAVE;
        let r = rota("/api/gerar", &format!("{base}&valor=abc"), false);
        assert_eq!(r.status, 400);
        assert!(corpo(&r).contains("valor inválido"));
        let r = rota("/api/gerar", &format!("{base}&txid=a%20b"), false);
        assert!(corpo(&r).contains("txid inválido"));
        let r = rota(
            "/api/gerar",
            &format!("{base}&txid=PED1&descricao=Pedido+1"),
            false,
        );
        assert_eq!(r.status, 200);
        assert!(corpo(&r).contains("62090505PED1") || corpo(&r).contains("PED1"));
        let longa = "d".repeat(80);
        let r = rota("/api/gerar", &format!("{base}&descricao={longa}"), false);
        assert!(corpo(&r).contains("descrição passa de 72"));
    }

    #[test]
    fn gerar_com_chave_ruim_da_400() {
        let r = rota("/api/gerar", "chave=abc&nome=Ana&cidade=Recife", false);
        assert_eq!(r.status, 400);
        assert!(corpo(&r).contains("chave Pix inválida"));
    }

    #[test]
    fn ler_devolve_os_campos_do_copia_e_cola() {
        let codigo = gerar(&parametros(CHAVE)).unwrap();
        let codigo_texto = {
            let inicio = codigo.find("\"codigo\":\"").unwrap() + 10;
            let fim = codigo[inicio..].find('"').unwrap() + inicio;
            codigo[inicio..fim].to_string()
        };
        let r = rota(
            "/api/ler",
            &format!("codigo={}", codigo_texto.replace(' ', "+")),
            false,
        );
        assert_eq!(r.status, 200, "{}", r.corpo);
        assert!(corpo(&r).contains("\"chave\":\"12345678909\""));
        assert!(corpo(&r).contains("\"nome\":\"Loja Exemplo\""));
        assert!(corpo(&r).contains("\"valor\":null"));
        assert!(corpo(&r).contains("\"campos\":[{\"id\":\"00\""));
    }

    #[test]
    fn ler_codigo_ruim_da_400_e_sem_codigo_tambem() {
        let r = rota("/api/ler", "codigo=000201", false);
        assert_eq!(r.status, 400);
        assert!(corpo(&r).contains("não li o código"));
        let r = rota("/api/ler", "", false);
        assert_eq!(r.status, 400);
        assert!(corpo(&r).contains("código Pix é obrigatório."));
    }

    #[test]
    fn rota_que_nao_existe_da_404_em_json() {
        let r = rota("/nao_existe", "", false);
        assert_eq!(r.status, 404);
        assert_eq!(r.tipo, JSON);
        assert!(corpo(&r).contains("Rota não encontrada."));
    }

    #[test]
    fn decodifica_mais_e_percentual() {
        assert_eq!(decodificar("a+b%20c"), "a b c");
        assert_eq!(decodificar("%41%42"), "AB");
        assert_eq!(decodificar("%2F"), "/");
        assert_eq!(decodificar("%+1"), "% 1");
        assert_eq!(decodificar("abc%"), "abc%");
        assert_eq!(decodificar("%4"), "%4");
        assert_eq!(decodificar("%zz"), "%zz");
        assert_eq!(decodificar("%é1"), "%é1");
        assert_eq!(decodificar("%ff"), "\u{fffd}");
    }

    #[test]
    fn parametros_aceita_chave_sem_valor_e_vazio() {
        assert_eq!(
            parametros("a=1&&b&c=x%3Dy"),
            vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), String::new()),
                ("c".to_string(), "x=y".to_string()),
            ]
        );
        assert_eq!(parametros(""), Vec::new());
    }

    #[test]
    fn texto_json_escapa_aspas_barras_e_controle() {
        assert_eq!(
            texto_json("a\"b\\c\nd\re\tf"),
            "\"a\\\"b\\\\c\\nd\\re\\tf\""
        );
        assert_eq!(texto_json("\u{1}"), "\"\\u0001\"");
        assert_eq!(texto_json("São"), "\"São\"");
    }

    #[test]
    fn opcional_json_vira_null_quando_nao_ha_valor() {
        assert_eq!(opcional_json(None), "null");
        assert_eq!(opcional_json(Some("x")), "\"x\"");
    }
}
