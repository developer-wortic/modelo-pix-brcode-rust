//! Monta o BR Code estático: o código de copia e cola do Pix com chave, valor opcional,
//! nome e cidade do recebedor, txid e descrição. O CRC16 vai no fim, no campo 63.

use crate::chave::Chave;
use crate::crc;
use crate::erro::Erro;
use crate::texto::{self, CIDADE_MAXIMA, DESCRICAO_MAXIMA, NOME_MAXIMO};
use crate::tlv;
use crate::valor;

/// O GUI do Pix, no subcampo 00 da conta (campo 26).
pub const GUI: &str = "br.gov.bcb.pix";

/// O txid padrão do Pix estático, quando não há txid.
pub const TXID_PADRAO: &str = "***";

/// Os dados de uma cobrança estática, já validados.
///
/// Os campos são públicos para leitura. Para montar, use `Cobranca::nova` e os métodos `com_*`,
/// que confere cada campo antes de deixar você gerar o código.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cobranca {
    pub chave: Chave,
    /// Valor em centavos. `None` quer dizer que quem paga digita o valor.
    pub valor: Option<u64>,
    pub nome: String,
    pub cidade: String,
    pub txid: Option<String>,
    pub descricao: Option<String>,
}

impl Cobranca {
    /// Começa uma cobrança com chave, nome e cidade do recebedor.
    ///
    /// # Errors
    ///
    /// Devolve `Erro::Chave` se a chave não passar na conferência, `Erro::Obrigatorio` se o nome
    /// ou a cidade vierem vazios, e `Erro::Tamanho` se passarem de 25 e 15 caracteres.
    pub fn nova(chave: &str, nome: &str, cidade: &str) -> Result<Self, Erro> {
        Ok(Self {
            chave: Chave::interpretar(chave)?,
            valor: None,
            nome: texto::obrigatorio("nome do recebedor", nome, NOME_MAXIMO)?,
            cidade: texto::obrigatorio("cidade", cidade, CIDADE_MAXIMA)?,
            txid: None,
            descricao: None,
        })
    }

    /// Põe o valor, digitado em reais (como "10,50").
    ///
    /// # Errors
    ///
    /// Devolve `Erro::Valor` se o texto não for um valor válido.
    pub fn com_valor(mut self, texto: &str) -> Result<Self, Erro> {
        self.valor = Some(valor::centavos(texto)?);
        Ok(self)
    }

    /// Põe o txid, de 1 a 25 letras e números.
    ///
    /// # Errors
    ///
    /// Devolve `Erro::TxId` se o txid não tiver a forma certa. Vazio fica sem txid.
    pub fn com_txid(mut self, txid: &str) -> Result<Self, Erro> {
        self.txid = texto::txid(txid)?;
        Ok(self)
    }

    /// Põe a descrição, que aparece no app do pagador. Passa de 72 caracteres é erro.
    ///
    /// # Errors
    ///
    /// Devolve `Erro::Tamanho` se a descrição passar de 72 caracteres.
    pub fn com_descricao(mut self, descricao: &str) -> Result<Self, Erro> {
        self.descricao = texto::opcional("descrição", descricao, DESCRICAO_MAXIMA)?;
        Ok(self)
    }

    /// Monta o BR Code como texto, com o CRC16 no fim.
    ///
    /// # Errors
    ///
    /// Devolve `Erro::Tamanho` se a chave e a descrição juntas passarem de 99 caracteres,
    /// que é o limite do campo 26.
    pub fn codigo(&self) -> Result<String, Erro> {
        let juntos = |_| Erro::Tamanho {
            campo: "a chave e a descrição juntas",
            maximo: tlv::MAXIMO_VALOR,
        };
        let mut conta = tlv::campo("00", GUI)?;
        conta.push_str(&tlv::campo("01", self.chave.valor())?);
        if let Some(descricao) = &self.descricao {
            conta.push_str(&tlv::campo("02", descricao)?);
        }
        let conta = tlv::campo("26", &conta).map_err(juntos)?;

        let mut corpo = String::new();
        corpo.push_str(&tlv::campo("00", "01")?);
        corpo.push_str(&tlv::campo("01", "11")?);
        corpo.push_str(&conta);
        corpo.push_str(&tlv::campo("52", "0000")?);
        corpo.push_str(&tlv::campo("53", "986")?);
        if let Some(centavos) = self.valor {
            corpo.push_str(&tlv::campo("54", &valor::texto(centavos))?);
        }
        corpo.push_str(&tlv::campo("58", "BR")?);
        corpo.push_str(&tlv::campo("59", &self.nome)?);
        corpo.push_str(&tlv::campo("60", &self.cidade)?);
        let txid = self.txid.as_deref().unwrap_or(TXID_PADRAO);
        corpo.push_str(&tlv::campo("62", &tlv::campo("05", txid)?)?);
        corpo.push_str("6304");
        let verificador = crc::crc16(corpo.as_bytes());
        Ok(format!("{corpo}{verificador:04X}"))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn monta_estatico_sem_valor_com_txid_padrao() {
        let c = Cobranca::nova("123.456.789-09", "Loja Exemplo", "Sao Paulo").unwrap();
        let codigo = c.codigo().unwrap();
        assert!(codigo.starts_with("000201010211"));
        assert!(codigo.contains("26330014br.gov.bcb.pix011112345678909"));
        assert!(codigo.contains("5802BR5912Loja Exemplo6009Sao Paulo62070503***6304"));
        assert_eq!(codigo.len(), codigo.find("6304").unwrap() + 8);
    }

    #[test]
    fn monta_com_valor_txid_e_descricao() {
        let codigo = Cobranca::nova("pix@exemplo.com.br", "Ana", "Recife")
            .unwrap()
            .com_valor("10,5")
            .unwrap()
            .com_txid("PED42")
            .unwrap()
            .com_descricao("Pedido 42")
            .unwrap()
            .codigo()
            .unwrap();
        assert!(codigo.contains("0118pix@exemplo.com.br"));
        assert!(codigo.contains("540510.50"));
        assert!(codigo.contains("0209Pedido 42"));
        assert!(codigo.contains("62090505PED42"));
    }

    #[test]
    fn nome_e_cidade_obrigatorios_e_com_limite() {
        assert_eq!(
            Cobranca::nova("123.456.789-09", " ", "Recife"),
            Err(Erro::Obrigatorio("nome do recebedor"))
        );
        assert_eq!(
            Cobranca::nova("123.456.789-09", "Ana", ""),
            Err(Erro::Obrigatorio("cidade"))
        );
        assert!(Cobranca::nova("123.456.789-09", &"n".repeat(26), "Recife").is_err());
        assert!(Cobranca::nova("123.456.789-09", "Ana", &"c".repeat(16)).is_err());
    }

    #[test]
    fn chave_e_descricao_juntas_nao_passam_de_99() {
        let email_longo = format!("{}@exemplo.com", "a".repeat(60));
        let c = Cobranca::nova(&email_longo, "Ana", "Recife")
            .unwrap()
            .com_descricao(&"d".repeat(40))
            .unwrap();
        assert_eq!(
            c.codigo(),
            Err(Erro::Tamanho {
                campo: "a chave e a descrição juntas",
                maximo: 99
            })
        );
    }

    #[test]
    fn descricao_passa_de_72_e_erro() {
        assert!(
            Cobranca::nova("123.456.789-09", "Ana", "Recife")
                .unwrap()
                .com_descricao(&"d".repeat(73))
                .is_err()
        );
    }

    #[test]
    fn valor_zero_e_txid_ruim_sao_erro() {
        let base = Cobranca::nova("123.456.789-09", "Ana", "Recife").unwrap();
        assert!(base.clone().com_valor("0").is_err());
        assert!(base.clone().com_txid("a b").is_err());
        assert_eq!(base.clone().com_txid("").unwrap().txid, None);
        assert!(base.com_valor("").is_err());
    }

    #[test]
    fn chave_ruim_nao_monta() {
        assert!(Cobranca::nova("123.456.789-00", "Ana", "Recife").is_err());
    }
}
