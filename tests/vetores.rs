//! Vetores fixos: o CRC conhecido e o código de um exemplo estático, com a estrutura campo a campo.
//! A saída do exemplo foi conferida com um CRC16-CCITT-FALSE feito à parte, em C.
//! O vetor oficial do manual do Banco Central ainda não está aqui: ver o README.

use pix_brcode::{Cobranca, crc, leitura};

#[test]
fn vetor_de_conferencia_do_crc_ccitt_false() {
    assert_eq!(crc::crc16(b"123456789"), 0x29B1);
}

#[test]
fn codigo_estatico_de_exemplo_sai_fixo() {
    let codigo = Cobranca::nova("123.456.789-09", "Loja Exemplo", "Sao Paulo")
        .unwrap()
        .codigo()
        .unwrap();
    assert_eq!(
        codigo,
        "00020101021126330014br.gov.bcb.pix0111123456789095204000053039865802BR5912Loja Exemplo6009Sao Paulo62070503***630467B2"
    );
}

#[test]
fn codigo_com_valor_txid_e_descricao_sai_fixo() {
    let codigo = Cobranca::nova("pix@exemplo.com.br", "Ana Maria", "Recife")
        .unwrap()
        .com_valor("25,90")
        .unwrap()
        .com_txid("PED7")
        .unwrap()
        .com_descricao("Pedido 7")
        .unwrap()
        .codigo()
        .unwrap();
    assert_eq!(
        codigo,
        "00020101021126520014br.gov.bcb.pix0118pix@exemplo.com.br0208Pedido 7520400005303986540525.905802BR5909Ana Maria6006Recife62080504PED76304B02D"
    );
}

#[test]
fn o_crc_do_codigo_confere_com_o_corpo() {
    let codigo = Cobranca::nova("123.456.789-09", "Loja Exemplo", "Sao Paulo")
        .unwrap()
        .codigo()
        .unwrap();
    let (corpo, fim) = codigo.split_at(codigo.len() - 4);
    assert!(corpo.ends_with("6304"));
    assert_eq!(format!("{:04X}", crc::crc16(corpo.as_bytes())), fim);
}

#[test]
fn a_leitura_do_vetor_devolve_os_campos_do_exemplo() {
    let codigo = Cobranca::nova("123.456.789-09", "Loja Exemplo", "Sao Paulo")
        .unwrap()
        .codigo()
        .unwrap();
    let l = leitura::ler(&codigo).unwrap();
    assert_eq!(l.chave, "12345678909");
    assert_eq!(l.nome, "Loja Exemplo");
    assert_eq!(l.cidade, "Sao Paulo");
    assert_eq!(l.valor, None);
    assert_eq!(l.txid.as_deref(), Some("***"));
}

/// Exemplo oficial do Manual de Padrões para Iniciação do Pix (Banco Central): chave aleatória, "Fulano de Tal",
/// Brasília, txid "***" e CRC 1D3D. A leitura tem de aceitar o código e devolver os campos como o manual escreve.
#[test]
fn exemplo_oficial_do_manual_do_banco_central() {
    let codigo = "00020126580014br.gov.bcb.pix0136123e4567-e12b-12d1-a456-4266554400005204000053039865802BR5913Fulano de Tal6008BRASILIA62070503***63041D3D";
    let l = leitura::ler(codigo).expect("o exemplo oficial tem de ser lido");
    assert_eq!(l.chave, "123e4567-e12b-12d1-a456-426655440000");
    assert_eq!(l.nome, "Fulano de Tal");
    assert_eq!(l.cidade, "BRASILIA");
    assert_eq!(l.moeda, "986");
    assert_eq!(l.pais, "BR");
    assert_eq!(l.categoria, "0000");
    assert_eq!(l.txid.as_deref(), Some("***"));
    assert_eq!(l.valor, None);
    assert_eq!(
        format!("{:04X}", crc::crc16(&codigo.as_bytes()[..codigo.len() - 4])),
        "1D3D"
    );
}
