//! O texto que entra no BR Code: sem acento, só letras, números e os sinais `.`, `-`, `/` e `&`,
//! com espaço simples e limite de caracteres por campo.

use crate::erro::Erro;

/// Limite do nome do recebedor (campo 59).
pub const NOME_MAXIMO: usize = 25;
/// Limite da cidade do recebedor (campo 60).
pub const CIDADE_MAXIMA: usize = 15;
/// Limite da descrição (campo 26, subcampo 02).
pub const DESCRICAO_MAXIMA: usize = 72;
/// Limite do txid (campo 62, subcampo 05).
pub const TXID_MAXIMO: usize = 25;

/// Põe o texto na forma ASCII aceita pelo BR Code: tira acento, troca espaço especial por espaço,
/// tira o que não é letra, número ou um dos sinais permitidos, e junta espaços repetidos.
#[must_use]
pub fn ascii(texto: &str) -> String {
    let mut saida = String::new();
    for c in texto.chars().map(sem_acento) {
        if c == ' ' {
            if !saida.is_empty() && !saida.ends_with(' ') {
                saida.push(' ');
            }
        } else if c.is_ascii_alphanumeric() || ".-/&".contains(c) {
            saida.push(c);
        }
    }
    saida.trim_end().to_string()
}

fn sem_acento(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'Á' | 'À' | 'Â' | 'Ã' | 'Ä' | 'Å' => 'A',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
        'ç' => 'c',
        'Ç' => 'C',
        'ñ' => 'n',
        'Ñ' => 'N',
        'ý' | 'ÿ' => 'y',
        'Ý' => 'Y',
        outro if outro.is_whitespace() => ' ',
        outro => outro,
    }
}

/// Um campo obrigatório: depois de normalizado, não pode ficar vazio nem passar do limite.
///
/// # Errors
///
/// Devolve `Erro::Obrigatorio` se ficar vazio e `Erro::Tamanho` se passar de `maximo`.
pub fn obrigatorio(campo: &'static str, texto: &str, maximo: usize) -> Result<String, Erro> {
    let valor = ascii(texto);
    if valor.is_empty() {
        return Err(Erro::Obrigatorio(campo));
    }
    confere_tamanho(campo, valor, maximo)
}

/// Um campo opcional: vazio vira `None`, e passar do limite é erro.
///
/// # Errors
///
/// Devolve `Erro::Tamanho` se o texto normalizado passar de `maximo`.
pub fn opcional(campo: &'static str, texto: &str, maximo: usize) -> Result<Option<String>, Erro> {
    let valor = ascii(texto);
    if valor.is_empty() {
        return Ok(None);
    }
    confere_tamanho(campo, valor, maximo).map(Some)
}

fn confere_tamanho(campo: &'static str, valor: String, maximo: usize) -> Result<String, Erro> {
    if valor.len() > maximo {
        Err(Erro::Tamanho { campo, maximo })
    } else {
        Ok(valor)
    }
}

/// O txid: de 1 a 25 letras e números, sem acento, espaço ou símbolo. Vazio vira `None`.
///
/// # Errors
///
/// Devolve `Erro::TxId` quando o txid tem outro caractere ou passa de 25.
pub fn txid(texto: &str) -> Result<Option<String>, Erro> {
    let t = texto.trim();
    if t.is_empty() {
        return Ok(None);
    }
    if t.len() > TXID_MAXIMO || !t.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(Erro::TxId(
            "use de 1 a 25 letras e números, sem espaço nem símbolo",
        ));
    }
    Ok(Some(t.to_string()))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn tira_acento_e_mantem_o_basico() {
        assert_eq!(ascii("Açaí & Cia. Ltda"), "Acai & Cia. Ltda");
        assert_eq!(ascii("Pão de Queijo"), "Pao de Queijo");
        assert_eq!(ascii("São Paulo"), "Sao Paulo");
        assert_eq!(ascii("Ñandú Ýpsilon"), "Nandu Ypsilon");
        assert_eq!(ascii("ÁÉÍÓÚÀÈÌÒÙÂÊÎÔÛÃÕËÏÜÇ"), "AEIOUAEIOUAEIOUAOEIUC");
        assert_eq!(ascii("áéíóúàèìòùâêîôûãõäëïöüç"), "aeiouaeiouaeiouaoaeiouc");
    }

    #[test]
    fn tira_simbolo_que_nao_entra_e_junta_espacos() {
        assert_eq!(ascii("  Loja   do   João!  "), "Loja do Joao");
        assert_eq!(ascii("A\u{a0}B\tC\nD"), "A B C D");
        assert_eq!(ascii("Oi?@#$%"), "Oi");
        assert_eq!(ascii("Ana/Bia-Cia"), "Ana/Bia-Cia");
        assert_eq!(ascii("emoji 😀 fim"), "emoji fim");
        assert_eq!(ascii("***"), "");
        assert_eq!(ascii("ñ ý ÿ Ñ Ý"), "n y y N Y");
    }

    #[test]
    fn obrigatorio_recusa_vazio_e_passa_do_limite() {
        assert_eq!(
            obrigatorio("cidade", " São Paulo ", 15),
            Ok("Sao Paulo".into())
        );
        assert_eq!(
            obrigatorio("cidade", "   ", 15),
            Err(Erro::Obrigatorio("cidade"))
        );
        assert_eq!(
            obrigatorio("cidade", "Rio de Janeiro Capital", 15),
            Err(Erro::Tamanho {
                campo: "cidade",
                maximo: 15
            })
        );
        assert_eq!(
            obrigatorio("nome", &"a".repeat(25), NOME_MAXIMO).map(|s| s.len()),
            Ok(25)
        );
        assert!(obrigatorio("nome", &"a".repeat(26), NOME_MAXIMO).is_err());
    }

    #[test]
    fn opcional_vazio_e_none_e_limite_vale() {
        assert_eq!(opcional("descrição", "  ", 72), Ok(None));
        assert_eq!(
            opcional("descrição", "Pedido 42", 72),
            Ok(Some("Pedido 42".into()))
        );
        assert!(opcional("descrição", &"x".repeat(73), 72).is_err());
    }

    #[test]
    fn txid_aceita_so_letras_e_numeros() {
        assert_eq!(txid(""), Ok(None));
        assert_eq!(txid(" PED123 "), Ok(Some("PED123".into())));
        assert_eq!(txid(&"A".repeat(25)), Ok(Some("A".repeat(25))));
        assert!(txid(&"A".repeat(26)).is_err());
        assert!(txid("PED 123").is_err());
        assert!(txid("PED-123").is_err());
        assert!(txid("Pão").is_err());
    }
}
