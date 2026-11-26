//! Lê um BR Code do Pix: confere o CRC16, separa os campos e devolve o que o pagamento diz.
//! Não consulta nada: só lê o texto que você colou.

use crate::cobranca::GUI;
use crate::crc;
use crate::erro::Erro;
use crate::tlv::{self, Campo};

/// O que sai de um BR Code lido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leitura {
    /// A chave Pix, como está no código (campo 26, subcampo 01).
    pub chave: String,
    /// A descrição, se o código tem uma (campo 26, subcampo 02).
    pub descricao: Option<String>,
    /// O valor no formato do código, como "10.50". `None` quando quem paga digita.
    pub valor: Option<String>,
    /// A moeda: sempre "986" (real), ou a leitura falha.
    pub moeda: String,
    /// O país: quase sempre "BR".
    pub pais: String,
    /// O nome do recebedor (campo 59).
    pub nome: String,
    /// A cidade do recebedor (campo 60).
    pub cidade: String,
    /// A categoria do comerciante (campo 52), normalmente "0000".
    pub categoria: String,
    /// O txid (campo 62, subcampo 05). Pix estático sem txid traz "***".
    pub txid: Option<String>,
    /// Todos os campos do primeiro nível, na ordem em que vieram.
    pub campos: Vec<Campo>,
}

/// Lê um BR Code colado. Espaços nas pontas são ignorados.
///
/// # Errors
///
/// Devolve `Erro::Leitura` quando o texto não é ASCII, quando o CRC do campo 63 não confere,
/// quando falta um campo obrigatório, quando o GUI não é `br.gov.bcb.pix`, quando a moeda não é 986,
/// ou quando o código é dinâmico (com URL no campo 26, subcampo 25).
pub fn ler(codigo: &str) -> Result<Leitura, Erro> {
    let t = codigo.trim();
    if !t.is_ascii() {
        return Err(Erro::Leitura("só cabem caracteres ASCII"));
    }
    let fim = t
        .len()
        .checked_sub(8)
        .ok_or(Erro::Leitura("código curto demais"))?;
    let verificador = &t[fim..];
    if !verificador.starts_with("6304") {
        return Err(Erro::Leitura("o código não termina com o campo 63 (CRC)"));
    }
    let informado = u16::from_str_radix(&verificador[4..], 16)
        .map_err(|_| Erro::Leitura("o CRC não é hexadecimal"))?;
    if crc::crc16(&t.as_bytes()[..t.len() - 4]) != informado {
        return Err(Erro::Leitura(
            "o CRC não confere: o código foi alterado ou digitado errado",
        ));
    }
    let campos = tlv::ler(t)?;
    if campos.last().is_none_or(|c| c.id != "63") {
        return Err(Erro::Leitura("o campo 63 não é o último"));
    }

    let conta = campo(&campos, "26").ok_or(Erro::Leitura("falta o campo 26 (conta Pix)"))?;
    let subcampos = tlv::ler(conta)?;
    let gui = campo(&subcampos, "00").ok_or(Erro::Leitura("falta o GUI da conta Pix"))?;
    if gui != GUI {
        return Err(Erro::Leitura("o GUI do campo 26 não é br.gov.bcb.pix"));
    }
    if campo(&subcampos, "25").is_some() {
        return Err(Erro::Leitura(
            "código dinâmico, com URL: esta leitura só cobre o estático",
        ));
    }
    let chave = campo(&subcampos, "01")
        .ok_or(Erro::Leitura("falta a chave Pix no campo 26"))?
        .to_string();

    let moeda = obrigatorio(&campos, "53", "falta o campo 53 (moeda)")?;
    if moeda != "986" {
        return Err(Erro::Leitura("a moeda não é 986 (real)"));
    }
    let adicionais = campo(&campos, "62").map(tlv::ler).transpose()?;
    let txid = adicionais
        .as_deref()
        .and_then(|lista| campo(lista, "05"))
        .map(str::to_string);

    Ok(Leitura {
        chave,
        descricao: campo(&subcampos, "02").map(str::to_string),
        valor: campo(&campos, "54").map(str::to_string),
        moeda,
        pais: obrigatorio(&campos, "58", "falta o campo 58 (país)")?,
        nome: obrigatorio(&campos, "59", "falta o campo 59 (nome)")?,
        cidade: obrigatorio(&campos, "60", "falta o campo 60 (cidade)")?,
        categoria: obrigatorio(&campos, "52", "falta o campo 52 (categoria)")?,
        txid,
        campos,
    })
}

fn campo<'a>(campos: &'a [Campo], id: &str) -> Option<&'a str> {
    campos.iter().find(|c| c.id == id).map(|c| c.valor.as_str())
}

fn obrigatorio(campos: &[Campo], id: &str, falta: &'static str) -> Result<String, Erro> {
    campo(campos, id)
        .map(str::to_string)
        .ok_or(Erro::Leitura(falta))
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::cobranca::Cobranca;

    /// Fecha um corpo com "6304" e o CRC certo, como um BR Code de verdade.
    fn fechar(corpo: &str) -> String {
        let com_tag = format!("{corpo}6304");
        format!("{com_tag}{:04X}", crc::crc16(com_tag.as_bytes()))
    }

    fn campo_teste(id: &'static str, valor: &str) -> String {
        tlv::campo(id, valor).unwrap()
    }

    /// Os campos obrigatórios de um BR Code, cada um com o tamanho certo.
    fn partes(conta: &str) -> Vec<String> {
        vec![
            campo_teste("00", "01"),
            campo_teste("01", "11"),
            campo_teste("26", conta),
            campo_teste("52", "0000"),
            campo_teste("53", "986"),
            campo_teste("58", "BR"),
            campo_teste("59", "Ana"),
            campo_teste("60", "Recife"),
            campo_teste("62", &campo_teste("05", "***")),
        ]
    }

    fn conta_padrao() -> String {
        [campo_teste("00", GUI), campo_teste("01", "123456789")].concat()
    }

    fn valido() -> String {
        partes(&conta_padrao()).concat()
    }

    fn sem(campos: Vec<String>, id: &str) -> String {
        campos.into_iter().filter(|p| !p.starts_with(id)).collect()
    }

    #[test]
    fn le_um_codigo_montado_pela_biblioteca() {
        let codigo = Cobranca::nova("123.456.789-09", "Ana Maria", "Recife")
            .unwrap()
            .com_valor("25,90")
            .unwrap()
            .com_txid("PED7")
            .unwrap()
            .com_descricao("Pedido 7")
            .unwrap()
            .codigo()
            .unwrap();
        let l = ler(&format!("  {codigo}  ")).unwrap();
        assert_eq!(l.chave, "12345678909");
        assert_eq!(l.descricao.as_deref(), Some("Pedido 7"));
        assert_eq!(l.valor.as_deref(), Some("25.90"));
        assert_eq!(l.moeda, "986");
        assert_eq!(l.pais, "BR");
        assert_eq!(l.nome, "Ana Maria");
        assert_eq!(l.cidade, "Recife");
        assert_eq!(l.categoria, "0000");
        assert_eq!(l.txid.as_deref(), Some("PED7"));
        assert_eq!(l.campos.len(), 11);
    }

    #[test]
    fn sem_valor_nem_descricao_e_txid_vem_como_asterisco() {
        let l = ler(&fechar(&valido())).unwrap();
        assert_eq!(l.valor, None);
        assert_eq!(l.descricao, None);
        assert_eq!(l.txid.as_deref(), Some("***"));
        assert_eq!(l.chave, "123456789");
    }

    #[test]
    fn sem_campo_62_o_txid_fica_none() {
        let corpo = sem(partes(&conta_padrao()), "62");
        assert_eq!(ler(&fechar(&corpo)).unwrap().txid, None);
    }

    #[test]
    fn campo_62_sem_subcampo_05_da_txid_none() {
        let corpo: String = partes(&conta_padrao())
            .into_iter()
            .map(|p| {
                if p.starts_with("62") {
                    campo_teste("62", &campo_teste("99", "x"))
                } else {
                    p
                }
            })
            .collect();
        assert_eq!(ler(&fechar(&corpo)).unwrap().txid, None);
    }

    #[test]
    fn recusa_crc_errado_ou_texto_alterado() {
        let codigo = fechar(&valido());
        let trocado = codigo.replace("Ana", "Bia");
        assert!(matches!(ler(&trocado), Err(Erro::Leitura(m)) if m.contains("CRC não confere")));
        let outro_final = format!("{}0000", &codigo[..codigo.len() - 4]);
        assert!(ler(&outro_final).is_err());
    }

    #[test]
    fn recusa_texto_que_nao_termina_em_crc() {
        assert_eq!(ler("0002"), Err(Erro::Leitura("código curto demais")));
        assert!(matches!(
            ler(&format!("{}6305ABCD", valido())),
            Err(Erro::Leitura(m)) if m.contains("não termina")
        ));
        assert!(matches!(
            ler(&format!("{}6304GGGG", valido())),
            Err(Erro::Leitura(m)) if m.contains("hexadecimal")
        ));
    }

    #[test]
    fn recusa_nao_ascii() {
        assert_eq!(ler("ção"), Err(Erro::Leitura("só cabem caracteres ASCII")));
    }

    #[test]
    fn recusa_campo_63_que_nao_e_o_ultimo() {
        // o último campo é o 99, cujo valor é "6304" mais um CRC válido: a conferência passa,
        // mas o campo 63 não é o último
        let corpo = format!("{}9908", valido());
        let com_tag = format!("{corpo}6304");
        let fechado = format!("{com_tag}{:04X}", crc::crc16(com_tag.as_bytes()));
        let r = ler(&fechado);
        assert!(
            matches!(&r, Err(Erro::Leitura(m)) if m.contains("não é o último")),
            "{r:?}"
        );
    }

    #[test]
    fn recusa_campo_26_ausente_ou_com_gui_errado() {
        let sem26 = sem(partes(&conta_padrao()), "26");
        assert!(matches!(
            ler(&fechar(&sem26)),
            Err(Erro::Leitura(m)) if m.contains("falta o campo 26")
        ));
        let gui_errado =
            partes(&[campo_teste("00", "br.gov.bcb"), campo_teste("01", "1")].concat()).concat();
        assert!(matches!(
            ler(&fechar(&gui_errado)),
            Err(Erro::Leitura(m)) if m.contains("GUI do campo 26")
        ));
        let sem_gui = partes(&campo_teste("01", "123456789")).concat();
        assert!(matches!(
            ler(&fechar(&sem_gui)),
            Err(Erro::Leitura(m)) if m.contains("falta o GUI")
        ));
    }

    #[test]
    fn recusa_codigo_dinamico_com_url() {
        let conta = [campo_teste("00", GUI), campo_teste("25", "https://x.y")].concat();
        assert!(matches!(
            ler(&fechar(&partes(&conta).concat())),
            Err(Erro::Leitura(m)) if m.contains("dinâmico")
        ));
    }

    #[test]
    fn recusa_sem_chave_moeda_errada_ou_sem_moeda() {
        let sem_chave = partes(&campo_teste("00", GUI)).concat();
        assert!(matches!(
            ler(&fechar(&sem_chave)),
            Err(Erro::Leitura(m)) if m.contains("falta a chave")
        ));
        let dolar: String = partes(&conta_padrao())
            .into_iter()
            .map(|p| {
                if p.starts_with("53") {
                    campo_teste("53", "840")
                } else {
                    p
                }
            })
            .collect();
        assert!(matches!(
            ler(&fechar(&dolar)),
            Err(Erro::Leitura(m)) if m.contains("986")
        ));
        let sem53 = sem(partes(&conta_padrao()), "53");
        assert!(matches!(
            ler(&fechar(&sem53)),
            Err(Erro::Leitura(m)) if m.contains("campo 53")
        ));
    }

    #[test]
    fn recusa_cada_campo_obrigatorio_que_falta() {
        for (id, msg) in [
            ("52", "campo 52"),
            ("58", "campo 58"),
            ("59", "campo 59"),
            ("60", "campo 60"),
        ] {
            let corpo = sem(partes(&conta_padrao()), id);
            let r = ler(&fechar(&corpo));
            assert!(
                matches!(&r, Err(Erro::Leitura(m)) if m.contains(msg)),
                "faltando {id}: {r:?}"
            );
        }
    }

    #[test]
    fn dados_adicionais_quebrados_sao_erro() {
        let quebrado: String = partes(&conta_padrao())
            .into_iter()
            .map(|p| {
                if p.starts_with("62") {
                    "6205xx".to_string()
                } else {
                    p
                }
            })
            .collect();
        assert!(ler(&fechar(&quebrado)).is_err());
    }

    #[test]
    fn campo_de_nivel_de_cima_quebrado_e_erro() {
        assert!(ler(&fechar("0009")).is_err());
    }
}
