//! Os erros da biblioteca, com mensagem em português.

use std::fmt;

/// Tudo que pode dar errado ao montar, validar ou ler um Pix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Erro {
    /// Um campo que não pode faltar veio vazio.
    Obrigatorio(&'static str),
    /// Um campo passa do limite de caracteres.
    Tamanho { campo: &'static str, maximo: usize },
    /// A chave Pix não passa na conferência de formato ou de dígito.
    Chave(&'static str),
    /// O valor em reais não é um número com até duas casas decimais, ou é zero, ou é grande demais.
    Valor(&'static str),
    /// O txid não tem de 1 a 25 letras e números.
    TxId(&'static str),
    /// O texto não se lê como BR Code do Pix.
    Leitura(&'static str),
    /// O texto é grande demais para caber num QR Code.
    Qr,
}

impl fmt::Display for Erro {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Erro::Obrigatorio(campo) => write!(f, "{campo} é obrigatório."),
            Erro::Tamanho { campo, maximo } => {
                write!(f, "{campo} passa de {maximo} caracteres.")
            }
            Erro::Chave(motivo) => write!(f, "chave Pix inválida: {motivo}."),
            Erro::Valor(motivo) => write!(f, "valor inválido: {motivo}."),
            Erro::TxId(motivo) => write!(f, "txid inválido: {motivo}."),
            Erro::Leitura(motivo) => write!(f, "não li o código: {motivo}."),
            Erro::Qr => write!(f, "o texto é grande demais para um QR Code."),
        }
    }
}

impl std::error::Error for Erro {}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cada_erro_tem_a_mensagem_em_portugues() {
        let casos = [
            (Erro::Obrigatorio("cidade"), "cidade é obrigatório."),
            (
                Erro::Tamanho {
                    campo: "nome",
                    maximo: 25,
                },
                "nome passa de 25 caracteres.",
            ),
            (
                Erro::Chave("CPF inválido"),
                "chave Pix inválida: CPF inválido.",
            ),
            (Erro::Valor("use números"), "valor inválido: use números."),
            (Erro::TxId("vazio"), "txid inválido: vazio."),
            (Erro::Leitura("sem CRC"), "não li o código: sem CRC."),
            (Erro::Qr, "o texto é grande demais para um QR Code."),
        ];
        for (erro, esperado) in casos {
            assert_eq!(erro.to_string(), esperado);
        }
    }
}
