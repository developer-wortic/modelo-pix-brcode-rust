//! TLV do EMV: cada campo é o id (2 dígitos), o tamanho do valor (2 dígitos, em decimal) e o valor.
//! Um campo pode conter outros campos, como o 26 (conta Pix) e o 62 (dados adicionais).

use crate::erro::Erro;

/// O maior valor que cabe num campo TLV, porque o tamanho tem dois dígitos.
pub const MAXIMO_VALOR: usize = 99;

/// Um campo lido de um TLV: o id e o valor, como vieram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Campo {
    pub id: String,
    pub valor: String,
}

/// Monta um campo: `id` + tamanho com dois dígitos + `valor`.
///
/// # Errors
///
/// Devolve `Erro::Tamanho` com o nome `id` se o valor passar de 99 caracteres.
pub fn campo(id: &'static str, valor: &str) -> Result<String, Erro> {
    if valor.len() > MAXIMO_VALOR {
        return Err(Erro::Tamanho {
            campo: id,
            maximo: MAXIMO_VALOR,
        });
    }
    Ok(format!("{id}{:02}{valor}", valor.len()))
}

/// Lê uma sequência de campos TLV. O texto precisa ser ASCII, porque o tamanho conta bytes.
///
/// # Errors
///
/// Devolve `Erro::Leitura` se o texto não for ASCII, se faltar um pedaço de cabeçalho,
/// se o id ou o tamanho não tiverem dois dígitos, ou se o valor for menor do que o tamanho anunciado.
pub fn ler(texto: &str) -> Result<Vec<Campo>, Erro> {
    if !texto.is_ascii() {
        return Err(Erro::Leitura("só cabem caracteres ASCII"));
    }
    let mut campos = Vec::new();
    let mut resto = texto;
    while !resto.is_empty() {
        if resto.len() < 4 {
            return Err(Erro::Leitura("o código acaba no meio de um campo"));
        }
        let (id, tamanho) = (&resto[..2], &resto[2..4]);
        if !id.bytes().all(|b| b.is_ascii_digit()) || !tamanho.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Erro::Leitura("id ou tamanho de campo sem dois dígitos"));
        }
        // os dois dígitos já foram conferidos acima, então a conta não tem erro possível
        let (dezena, unidade) = (
            usize::from(tamanho.as_bytes()[0] - b'0'),
            usize::from(tamanho.as_bytes()[1] - b'0'),
        );
        let fim = 4 + dezena * 10 + unidade;
        if resto.len() < fim {
            return Err(Erro::Leitura("um valor é menor do que o tamanho anunciado"));
        }
        campos.push(Campo {
            id: id.to_string(),
            valor: resto[4..fim].to_string(),
        });
        resto = &resto[fim..];
    }
    Ok(campos)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn monta_com_tamanho_de_dois_digitos() {
        assert_eq!(campo("00", "01").unwrap(), "000201");
        assert_eq!(campo("59", "Fulano").unwrap(), "5906Fulano");
        assert_eq!(campo("00", "").unwrap(), "0000");
    }

    #[test]
    fn valor_de_99_passa_e_de_100_nao() {
        assert_eq!(campo("02", &"a".repeat(99)).unwrap().len(), 4 + 99);
        assert_eq!(
            campo("02", &"a".repeat(100)),
            Err(Erro::Tamanho {
                campo: "02",
                maximo: 99
            })
        );
    }

    #[test]
    fn le_a_sequencia_de_volta() {
        let texto = [
            "000201",
            "2618",
            "0014br.gov.bcb.pix",
            "540510.00",
            "5802BR",
            "6304ABCD",
        ]
        .concat();
        let campos = ler(&texto).unwrap();
        let ids: Vec<&str> = campos.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["00", "26", "54", "58", "63"]);
        assert_eq!(campos[1].valor, "0014br.gov.bcb.pix");
        assert_eq!(campos[2].valor, "10.00");
    }

    #[test]
    fn le_texto_vazio_como_nenhum_campo() {
        assert_eq!(ler("").unwrap(), Vec::new());
    }

    #[test]
    fn recusa_cabecalho_cortado_e_valor_curto() {
        assert!(ler("0001").is_err());
        assert!(ler("000").is_err());
        assert!(ler("00").is_err());
        assert!(ler("0005ab").is_err());
        assert!(ler("00xx01").is_err());
        assert!(ler("0A0101").is_err());
        assert!(ler("00ab").is_err());
    }

    #[test]
    fn recusa_texto_que_nao_e_ascii() {
        assert_eq!(
            ler("00ção"),
            Err(Erro::Leitura("só cabem caracteres ASCII"))
        );
    }
}
