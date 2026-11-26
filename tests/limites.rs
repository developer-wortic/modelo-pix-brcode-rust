//! Limites de tamanho de cada campo, e os casos de borda de acento, centavos e txid.

use pix_brcode::{Cobranca, Erro, leitura, texto, valor};

const CPF: &str = "123.456.789-09";

#[test]
fn nome_do_recebedor_aceita_25_e_recusa_26() {
    assert!(Cobranca::nova(CPF, &"a".repeat(25), "Recife").is_ok());
    assert_eq!(
        Cobranca::nova(CPF, &"a".repeat(26), "Recife"),
        Err(Erro::Tamanho {
            campo: "nome do recebedor",
            maximo: 25
        })
    );
}

#[test]
fn nome_conta_depois_de_tirar_acento_e_simbolo() {
    // 25 letras acentuadas continuam com 25 depois de normalizadas
    let acentuado = "ç".repeat(25);
    let c = Cobranca::nova(CPF, &acentuado, "Recife").unwrap();
    assert_eq!(c.nome, "c".repeat(25));
    // símbolo que não entra no BR Code some e não conta
    let com_simbolo = format!("{}!!!!", "a".repeat(25));
    assert_eq!(
        Cobranca::nova(CPF, &com_simbolo, "Recife")
            .unwrap()
            .nome
            .len(),
        25
    );
}

#[test]
fn cidade_aceita_15_e_recusa_16() {
    assert!(Cobranca::nova(CPF, "Ana", &"c".repeat(15)).is_ok());
    assert_eq!(
        Cobranca::nova(CPF, "Ana", &"c".repeat(16)),
        Err(Erro::Tamanho {
            campo: "cidade",
            maximo: 15
        })
    );
}

#[test]
fn descricao_aceita_72_e_recusa_73() {
    let base = Cobranca::nova(CPF, "Ana", "Recife").unwrap();
    assert!(base.clone().com_descricao(&"d".repeat(72)).is_ok());
    assert!(base.com_descricao(&"d".repeat(73)).is_err());
}

#[test]
fn txid_aceita_25_e_recusa_26() {
    let base = Cobranca::nova(CPF, "Ana", "Recife").unwrap();
    let ok = base.clone().com_txid(&"Z".repeat(25)).unwrap();
    assert_eq!(ok.txid.as_deref(), Some("ZZZZZZZZZZZZZZZZZZZZZZZZZ"));
    assert!(base.com_txid(&"Z".repeat(26)).is_err());
}

#[test]
fn valor_com_centavos_e_limite_do_campo_54() {
    let base = Cobranca::nova(CPF, "Ana", "Recife").unwrap();
    let codigo = base.clone().com_valor("1234,56").unwrap().codigo().unwrap();
    assert!(codigo.contains("54071234.56"));
    assert_eq!(
        leitura::ler(&codigo).unwrap().valor.as_deref(),
        Some("1234.56")
    );

    let maximo = base
        .clone()
        .com_valor("9999999999,99")
        .unwrap()
        .codigo()
        .unwrap();
    assert!(maximo.contains("54139999999999.99"));
    assert_eq!(
        leitura::ler(&maximo).unwrap().valor.as_deref(),
        Some("9999999999.99")
    );
    assert!(base.com_valor("10000000000,00").is_err());
}

#[test]
fn centavos_viram_texto_com_duas_casas() {
    assert_eq!(texto::ascii("São José"), "Sao Jose");
    assert_eq!(valor::texto(5), "0.05");
    assert_eq!(valor::texto(120_000), "1200.00");
}

#[test]
fn chave_e_descricao_juntas_no_limite_de_99() {
    // e-mail de 77 caracteres ocupa 18 + 4 + 77 = 99 no campo 26: passa sem descrição...
    let local = "a".repeat(65);
    let email = format!("{local}@exemplo.com");
    assert_eq!(email.len(), 77);
    let sem = Cobranca::nova(&email, "Ana", "Recife").unwrap();
    assert!(sem.codigo().is_ok());
    // ...e qualquer descrição estoura o campo 26
    let com = sem.com_descricao("x").unwrap();
    assert!(matches!(
        com.codigo(),
        Err(Erro::Tamanho {
            campo: "a chave e a descrição juntas",
            maximo: 99
        })
    ));
}

#[test]
fn acentos_no_txid_sao_recusados_e_no_nome_viram_ascii() {
    let base = Cobranca::nova(CPF, "Ação", "Brasília").unwrap();
    assert_eq!(base.nome, "Acao");
    assert_eq!(base.cidade, "Brasilia");
    assert!(base.clone().com_txid("PÃO").is_err());
}
