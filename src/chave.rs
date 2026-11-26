//! As chaves Pix: CPF, CNPJ (inclusive o alfanumérico de 2026), e-mail, telefone com +55 e chave aleatória.
//! Tudo offline: confere a forma e os dígitos verificadores, sem consultar ninguém.

use crate::erro::Erro;

/// Tamanho máximo de uma chave Pix, em caracteres.
pub const MAXIMO: usize = 77;

/// O tipo da chave, do jeito que o Pix a classifica.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tipo {
    Cpf,
    Cnpj,
    Email,
    Telefone,
    Aleatoria,
}

impl Tipo {
    /// Nome do tipo para a tela e para a API.
    #[must_use]
    pub fn nome(self) -> &'static str {
        match self {
            Tipo::Cpf => "CPF",
            Tipo::Cnpj => "CNPJ",
            Tipo::Email => "e-mail",
            Tipo::Telefone => "telefone",
            Tipo::Aleatoria => "chave aleatória",
        }
    }
}

/// Uma chave Pix já conferida e normalizada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chave {
    tipo: Tipo,
    valor: String,
}

impl Chave {
    /// Reconhece o tipo pela forma do texto e devolve a chave normalizada.
    ///
    /// Com `@` é e-mail, com `+` é telefone, com a forma de UUID é chave aleatória, e
    /// o resto é lido como CPF (11 dígitos) ou CNPJ (14 caracteres). Pontos, traços e barras são ignorados.
    ///
    /// # Errors
    ///
    /// Devolve `Erro::Chave` quando o texto não tem forma de chave ou o dígito verificador não confere.
    pub fn interpretar(texto: &str) -> Result<Self, Erro> {
        let t = texto.trim();
        if t.contains('@') {
            let valor = t.to_ascii_lowercase();
            return if email_valido(&valor) {
                Ok(Self::nova(Tipo::Email, valor))
            } else {
                Err(Erro::Chave("e-mail fora do formato"))
            };
        }
        if t.starts_with('+') {
            let valor: String = t
                .chars()
                .filter(|c| !matches!(c, ' ' | '-' | '(' | ')'))
                .collect();
            return if telefone_valido(&valor) {
                Ok(Self::nova(Tipo::Telefone, valor))
            } else {
                Err(Erro::Chave(
                    "telefone precisa ser +55, DDD e número (8 ou 9 dígitos)",
                ))
            };
        }
        if forma_de_uuid(t) {
            let valor = t.to_ascii_lowercase();
            return if aleatoria_valida(&valor) {
                Ok(Self::nova(Tipo::Aleatoria, valor))
            } else {
                Err(Erro::Chave("chave aleatória precisa ser UUID versão 4"))
            };
        }
        let limpo = limpar(t);
        if limpo.len() == 11 && limpo.bytes().all(|b| b.is_ascii_digit()) {
            return if cpf_valido(&limpo) {
                Ok(Self::nova(Tipo::Cpf, limpo))
            } else {
                Err(Erro::Chave("CPF inválido: confira os dígitos"))
            };
        }
        if limpo.len() == 14 {
            return if cnpj_valido(&limpo) {
                Ok(Self::nova(Tipo::Cnpj, limpo))
            } else {
                Err(Erro::Chave("CNPJ inválido: confira os dígitos"))
            };
        }
        Err(Erro::Chave(
            "não parece CPF, CNPJ, e-mail, telefone ou chave aleatória",
        ))
    }

    fn nova(tipo: Tipo, valor: String) -> Self {
        Self { tipo, valor }
    }

    /// O tipo da chave.
    #[must_use]
    pub fn tipo(&self) -> Tipo {
        self.tipo
    }

    /// O valor normalizado, que vai no campo 26 do BR Code.
    #[must_use]
    pub fn valor(&self) -> &str {
        &self.valor
    }
}

/// Tira tudo que não é letra ou número e põe as letras em maiúsculas.
fn limpar(texto: &str) -> String {
    texto
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// A forma de UUID (8-4-4-4-12 com traços), sem conferir a versão.
fn forma_de_uuid(texto: &str) -> bool {
    texto.len() == 36 && texto.bytes().filter(|&b| b == b'-').count() == 4
}

/// UUID versão 4 em minúsculas: versão 4 no 15º caractere e variante 8, 9, a ou b no 20º.
fn aleatoria_valida(valor: &str) -> bool {
    let b = valor.as_bytes();
    forma_de_uuid(valor)
        && [8, 13, 18, 23].iter().all(|&i| b[i] == b'-')
        && b.iter()
            .enumerate()
            .filter(|(i, _)| ![8, 13, 18, 23].contains(i))
            .all(|(_, c)| c.is_ascii_digit() || (b'a'..=b'f').contains(c))
        && b[14] == b'4'
        && matches!(b[19], b'8' | b'9' | b'a' | b'b')
}

/// E-mail com um só `@`, usuário com letras, números e `._%+-`, domínio com ponto e
/// rótulos válidos, e no máximo 77 caracteres.
fn email_valido(valor: &str) -> bool {
    if valor.len() > MAXIMO || !valor.is_ascii() {
        return false;
    }
    let Some((usuario, dominio)) = valor.split_once('@') else {
        return false;
    };
    let usuario_ok = !usuario.is_empty()
        && usuario
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._%+-".contains(&b));
    let rotulos: Vec<&str> = dominio.split('.').collect();
    let rotulo_ok = |r: &&str| {
        !r.is_empty()
            && !r.starts_with('-')
            && !r.ends_with('-')
            && r.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    };
    let dominio_ok = rotulos.len() >= 2
        && rotulos.iter().all(rotulo_ok)
        && rotulos
            .last()
            .is_some_and(|tld| tld.len() >= 2 && tld.bytes().all(|b| b.is_ascii_alphabetic()));
    usuario_ok && dominio_ok
}

/// Telefone: `+55`, depois DDD (que não começa com 0) e 8 dígitos, ou 9 dígitos começando com 9.
fn telefone_valido(valor: &str) -> bool {
    let Some(resto) = valor.strip_prefix("+55") else {
        return false;
    };
    if !resto.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let b = resto.as_bytes();
    match b.len() {
        10 => b[0] != b'0',
        11 => b[0] != b'0' && b[2] == b'9',
        _ => false,
    }
}

/// CPF: 11 dígitos, sem todos iguais, com os dois dígitos verificadores certos.
#[must_use]
pub fn cpf_valido(cpf: &str) -> bool {
    const PESOS_1: [u32; 9] = [10, 9, 8, 7, 6, 5, 4, 3, 2];
    const PESOS_2: [u32; 10] = [11, 10, 9, 8, 7, 6, 5, 4, 3, 2];
    let b = cpf.as_bytes();
    if b.len() != 11 || !cpf.bytes().all(|c| c.is_ascii_digit()) || cpf.bytes().all(|c| c == b[0]) {
        return false;
    }
    let digito = |c: u8| u32::from(c) - u32::from(b'0');
    let soma1: u32 = (0..9).map(|i| digito(b[i]) * PESOS_1[i]).sum();
    let dv1 = digito_cpf(soma1);
    let soma2: u32 = (0..9).map(|i| digito(b[i]) * PESOS_2[i]).sum::<u32>() + dv1 * PESOS_2[9];
    digito(b[9]) == dv1 && digito(b[10]) == digito_cpf(soma2)
}

fn digito_cpf(soma: u32) -> u32 {
    let resto = (soma * 10) % 11;
    if resto == 10 { 0 } else { resto }
}

/// CNPJ de 14 caracteres: números de 0 a 9 e letras de A a Z nas 12 primeiras posições
/// (valor de cada caractere é o código ASCII menos 48), e dois dígitos verificadores.
#[must_use]
pub fn cnpj_valido(cnpj: &str) -> bool {
    const PESOS_1: [u32; 12] = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    const PESOS_2: [u32; 13] = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let b = cnpj.as_bytes();
    if b.len() != 14 || cnpj.bytes().all(|c| c == b[0]) {
        return false;
    }
    let corpo_ok = b[..12]
        .iter()
        .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase());
    let dv_ok = b[12..].iter().all(u8::is_ascii_digit);
    if !corpo_ok || !dv_ok {
        return false;
    }
    let valor = |c: u8| u32::from(c) - u32::from(b'0');
    let soma1: u32 = (0..12).map(|i| valor(b[i]) * PESOS_1[i]).sum();
    let dv1 = digito_cnpj(soma1);
    let soma2: u32 = (0..12).map(|i| valor(b[i]) * PESOS_2[i]).sum::<u32>() + dv1 * PESOS_2[12];
    valor(b[12]) == dv1 && valor(b[13]) == digito_cnpj(soma2)
}

fn digito_cnpj(soma: u32) -> u32 {
    let resto = soma % 11;
    if resto < 2 { 0 } else { 11 - resto }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cpf_com_digito_certo_passa_e_com_digito_errado_nao() {
        assert!(cpf_valido("12345678909"));
        assert!(cpf_valido("52998224725"));
        assert!(!cpf_valido("12345678900"));
        assert!(!cpf_valido("12345678919"));
    }

    #[test]
    fn cpf_com_todos_os_digitos_iguais_nao_passa() {
        assert!(!cpf_valido("11111111111"));
        assert!(!cpf_valido("00000000000"));
    }

    #[test]
    fn cpf_com_tamanho_errado_nao_passa() {
        assert!(!cpf_valido("1234567890"));
        assert!(!cpf_valido("123456789090"));
        assert!(!cpf_valido("1234567890A"));
    }

    #[test]
    fn cnpj_numerico_e_alfanumerico_de_2026() {
        assert!(cnpj_valido("11222333000181"));
        assert!(cnpj_valido("12ABC34501DE35"));
        assert!(cnpj_valido("ABCDEFGHIJKL80"));
        assert!(!cnpj_valido("11222333000182"));
        assert!(!cnpj_valido("12ABC34501DE34"));
    }

    #[test]
    fn cnpj_com_forma_errada_nao_passa() {
        assert!(!cnpj_valido("00000000000000"));
        assert!(!cnpj_valido("12abc34501de35"));
        assert!(!cnpj_valido("12ABC34501DEA5"));
        assert!(!cnpj_valido("12ABC3450!DE35"));
        assert!(!cnpj_valido("1234567800019"));
    }

    #[test]
    fn digito_do_cnpj_com_resto_menor_que_dois_vira_zero() {
        assert_eq!(digito_cnpj(0), 0);
        assert_eq!(digito_cnpj(11), 0);
        assert_eq!(digito_cnpj(12), 0);
        assert_eq!(digito_cnpj(13), 9);
    }

    #[test]
    fn email_com_formato_e_limite() {
        assert!(email_valido("pix@exemplo.com.br"));
        assert!(email_valido("a.b_c%d+e-f@x-y.io"));
        assert!(!email_valido("sem-arroba.com"));
        assert!(!email_valido("@exemplo.com"));
        assert!(!email_valido("a@b@exemplo.com"));
        assert!(!email_valido("a@exemplo"));
        assert!(!email_valido("a@exemplo.c"));
        assert!(!email_valido("a@exemplo.c0m"));
        assert!(!email_valido("a@-exemplo.com"));
        assert!(!email_valido("a@exemplo-.com"));
        assert!(!email_valido("a@exemplo..com"));
        assert!(!email_valido("a@.com"));
        assert!(!email_valido("a b@exemplo.com"));
        assert!(!email_valido("ção@exemplo.com"));
        let longo = format!("{}@exemplo.com", "a".repeat(70));
        assert!(!email_valido(&longo));
    }

    #[test]
    fn telefone_com_mais_55_e_forma_certa() {
        assert!(telefone_valido("+5511999998888"));
        assert!(telefone_valido("+551133334444"));
        assert!(!telefone_valido("11999998888"));
        assert!(!telefone_valido("+5501999998888"));
        assert!(!telefone_valido("+5511899998888"));
        assert!(!telefone_valido("+55119999988"));
        assert!(!telefone_valido("+55119999988881"));
        assert!(!telefone_valido("+55a1999998888"));
        assert!(!telefone_valido("+1 5511999998888"));
    }

    #[test]
    fn aleatoria_precisa_ser_uuid_v4() {
        assert!(aleatoria_valida("123e4567-e89b-42d3-a456-426614174000"));
        assert!(!aleatoria_valida("123e4567-e89b-12d1-a456-426614174000"));
        assert!(!aleatoria_valida("123e4567-e89b-42d3-7456-426614174000"));
        assert!(!aleatoria_valida("123e4567-e89b-42d3-a456-42661417400g"));
        assert!(!aleatoria_valida("123e4567e89b42d3a456426614174000----"));
    }

    #[test]
    fn interpretar_reconhece_cada_tipo() {
        assert_eq!(
            Chave::interpretar("123.456.789-09").unwrap().tipo(),
            Tipo::Cpf
        );
        assert_eq!(
            Chave::interpretar("11.222.333/0001-81").unwrap().valor(),
            "11222333000181"
        );
        assert_eq!(
            Chave::interpretar(" Pix@Exemplo.com.br ").unwrap().valor(),
            "pix@exemplo.com.br"
        );
        assert_eq!(
            Chave::interpretar("+55 (11) 99999-8888").unwrap().valor(),
            "+5511999998888"
        );
        assert_eq!(
            Chave::interpretar("123E4567-E89B-42D3-A456-426614174000")
                .unwrap()
                .tipo(),
            Tipo::Aleatoria
        );
    }

    #[test]
    fn interpretar_recusa_o_que_nao_e_chave() {
        assert!(Chave::interpretar("").is_err());
        assert!(Chave::interpretar("abc").is_err());
        assert!(Chave::interpretar("12345").is_err());
        assert!(Chave::interpretar("123.456.789-00").is_err());
        assert!(Chave::interpretar("11.222.333/0001-82").is_err());
        assert!(Chave::interpretar("exemplo@").is_err());
    }

    #[test]
    fn nome_do_tipo_para_a_tela() {
        assert_eq!(Tipo::Cpf.nome(), "CPF");
        assert_eq!(Tipo::Cnpj.nome(), "CNPJ");
        assert_eq!(Tipo::Email.nome(), "e-mail");
        assert_eq!(Tipo::Telefone.nome(), "telefone");
        assert_eq!(Tipo::Aleatoria.nome(), "chave aleatória");
    }
}
