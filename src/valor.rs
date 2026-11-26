//! O valor em reais: de texto ("10,50" ou "10.50") para centavos, e de volta para o formato do BR Code ("10.50").

use crate::erro::Erro;

/// Maior valor que cabe no campo 54 (13 caracteres, como "9999999999.99").
pub const MAXIMO_CENTAVOS: u64 = 999_999_999_999;

/// Lê o valor digitado. Aceita vírgula ou ponto como separador decimal, com até duas casas.
/// Não aceita separador de milhar: "1.000" dá erro, para não virar R$ 1,00 por engano.
///
/// # Errors
///
/// Devolve `Erro::Valor` para texto sem número, mais de duas casas, zero ou valor acima de `MAXIMO_CENTAVOS`.
pub fn centavos(texto: &str) -> Result<u64, Erro> {
    let t = texto.trim();
    let (inteiro, decimal) = match t.find([',', '.']) {
        Some(i) => {
            let (antes, depois) = t.split_at(i);
            (antes, &depois[1..])
        }
        None => (t, ""),
    };
    let so_digitos = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !so_digitos(inteiro) {
        return Err(Erro::Valor(
            "use números, com vírgula ou ponto para os centavos",
        ));
    }
    if t.contains([',', '.']) && !so_digitos(decimal) {
        return Err(Erro::Valor(
            "depois da vírgula ou do ponto, use um a dois dígitos",
        ));
    }
    if decimal.len() > 2 {
        return Err(Erro::Valor("use no máximo duas casas decimais"));
    }
    let grande = || Erro::Valor("valor grande demais");
    let reais: u64 = inteiro.parse().map_err(|_| grande())?;
    // os dígitos já foram conferidos: a primeira casa vale dezena, a segunda vale unidade
    let frac: u64 = decimal
        .bytes()
        .enumerate()
        .map(|(i, b)| u64::from(b - b'0') * if i == 0 { 10 } else { 1 })
        .sum();
    let total = reais
        .checked_mul(100)
        .and_then(|v| v.checked_add(frac))
        .ok_or_else(grande)?;
    if total == 0 {
        return Err(Erro::Valor("o valor precisa ser maior que zero"));
    }
    if total > MAXIMO_CENTAVOS {
        return Err(grande());
    }
    Ok(total)
}

/// Os centavos no formato do BR Code: reais, ponto e dois dígitos (como "10.50").
#[must_use]
pub fn texto(centavos: u64) -> String {
    format!("{}.{:02}", centavos / 100, centavos % 100)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn aceita_virgula_e_ponto_com_centavos() {
        assert_eq!(centavos("10,50"), Ok(1050));
        assert_eq!(centavos("10.50"), Ok(1050));
        assert_eq!(centavos("0,01"), Ok(1));
        assert_eq!(centavos("10,5"), Ok(1050));
        assert_eq!(centavos("10"), Ok(1000));
        assert_eq!(centavos("  7,90  "), Ok(790));
    }

    #[test]
    fn recusa_milhar_zero_negativo_e_letra() {
        assert!(centavos("1.000").is_err());
        assert!(centavos("1.000,00").is_err());
        assert!(centavos("0").is_err());
        assert!(centavos("0,00").is_err());
        assert!(centavos("-5").is_err());
        assert!(centavos("cinco").is_err());
        assert!(centavos("").is_err());
        assert!(centavos("10,").is_err());
        assert!(centavos(",50").is_err());
        assert!(centavos("1,2a").is_err());
        assert!(centavos("10,500").is_err());
        // cabe em u64, mas estoura ao virar centavos
        assert!(centavos("1000000000000000000").is_err());
    }

    #[test]
    fn limite_do_campo_54() {
        assert_eq!(centavos("9999999999,99"), Ok(MAXIMO_CENTAVOS));
        assert!(centavos("10000000000,00").is_err());
        assert!(centavos("99999999999999999999999").is_err());
    }

    #[test]
    fn formata_com_ponto_e_dois_digitos() {
        assert_eq!(texto(1), "0.01");
        assert_eq!(texto(1050), "10.50");
        assert_eq!(texto(100), "1.00");
        assert_eq!(texto(MAXIMO_CENTAVOS), "9999999999.99");
    }
}
