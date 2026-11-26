//! Ida e volta: monta o código, lê de volta e confere cada campo, para centenas de casos gerados.
//! O gerador é determinístico (xorshift com semente fixa), então a suíte roda igual em qualquer máquina.

use pix_brcode::{Cobranca, Erro, crc, leitura, texto, valor};

struct Gerador(u64);

impl Gerador {
    fn proximo(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn ate(&mut self, n: u64) -> u64 {
        self.proximo() % n
    }

    fn chance(&mut self, em_cem: u64) -> bool {
        self.ate(100) < em_cem
    }

    fn escolhe<'a>(&mut self, itens: &[&'a str]) -> &'a str {
        let i = usize::try_from(self.ate(u64::try_from(itens.len()).unwrap())).unwrap();
        itens[i]
    }

    fn texto_de(&mut self, alfabeto: &[u8], tamanho: usize) -> String {
        let n = u64::try_from(alfabeto.len()).unwrap();
        (0..tamanho)
            .map(|_| char::from(alfabeto[usize::try_from(self.ate(n)).unwrap()]))
            .collect()
    }
}

const PALAVRAS: &[&str] = &[
    "Açaí", "Padaria", "do", "João", "Ônibus", "Cia", "Ltda", "Ateliê", "Pão", "de", "Queijo",
    "São", "Paulo", "Maçã", "Ação", "Rede", "Loja", "Exemplo", "Café", "Mãe",
];

/// Um nome de 1 a `maximo` palavras. Nome e cidade ficam dentro dos limites na maioria dos casos.
fn nome_gerado(g: &mut Gerador, maximo: u64) -> String {
    let n = 1 + usize::try_from(g.ate(maximo)).unwrap();
    (0..n)
        .map(|_| g.escolhe(PALAVRAS))
        .collect::<Vec<_>>()
        .join(" ")
}

fn cpf_valido(g: &mut Gerador) -> String {
    let mut d: Vec<u32> = (0..9).map(|_| u32::try_from(g.ate(10)).unwrap()).collect();
    if d.iter().all(|&x| x == d[0]) {
        d[0] = (d[0] + 1) % 10;
    }
    let dv = |d: &[u32], pesos: &[u32]| {
        let s: u32 = d.iter().zip(pesos).map(|(a, b)| a * b).sum();
        let r = (s * 10) % 11;
        if r == 10 { 0 } else { r }
    };
    let v1 = dv(&d, &[10, 9, 8, 7, 6, 5, 4, 3, 2]);
    d.push(v1);
    let v2 = dv(&d, &[11, 10, 9, 8, 7, 6, 5, 4, 3, 2]);
    d.push(v2);
    d.iter()
        .map(|x| char::from_digit(*x, 10).unwrap())
        .collect()
}

fn cnpj_valido(g: &mut Gerador) -> String {
    let alfabeto = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut corpo: Vec<u32> = (0..12)
        .map(|_| u32::from(alfabeto[usize::try_from(g.ate(36)).unwrap()]) - 48)
        .collect();
    let dv = |d: &[u32], pesos: &[u32]| {
        let s: u32 = d.iter().zip(pesos).map(|(a, b)| a * b).sum();
        let r = s % 11;
        if r < 2 { 0 } else { 11 - r }
    };
    let v1 = dv(&corpo, &[5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]);
    corpo.push(v1);
    let v2 = dv(&corpo, &[6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]);
    corpo.push(v2);
    corpo
        .iter()
        .map(|&x| char::from(u8::try_from(x + 48).unwrap()))
        .collect()
}

fn email_gerado(g: &mut Gerador) -> String {
    let tamanho = 3 + usize::try_from(g.ate(12)).unwrap();
    let usuario = g.texto_de(b"abcdefghijklmnopqrstuvwxyz0123456789._", tamanho);
    let dominio = g.escolhe(&["exemplo.com.br", "pagamentos.io", "loja-teste.com", "x.org"]);
    format!("{usuario}@{dominio}")
}

fn telefone_gerado(g: &mut Gerador) -> String {
    let ddd = 11 + g.ate(78);
    let celular = g.chance(70);
    let numero = if celular {
        format!("9{}", g.texto_de(b"0123456789", 8))
    } else {
        let primeiro = g.texto_de(b"23456789", 1);
        primeiro + &g.texto_de(b"0123456789", 7)
    };
    format!("+55{ddd}{numero}")
}

fn uuid_v4(g: &mut Gerador) -> String {
    let mut bytes = g.texto_de(b"0123456789abcdef", 36).into_bytes();
    for i in [8usize, 13, 18, 23] {
        bytes[i] = b'-';
    }
    bytes[14] = b'4';
    bytes[19] = b"89ab"[usize::try_from(g.ate(4)).unwrap()];
    String::from_utf8(bytes).unwrap()
}

fn chave_gerada(g: &mut Gerador) -> String {
    match g.ate(5) {
        0 => cpf_valido(g),
        1 => cnpj_valido(g),
        2 => email_gerado(g),
        3 => telefone_gerado(g),
        _ => uuid_v4(g),
    }
}

fn valor_gerado(g: &mut Gerador) -> Option<String> {
    if !g.chance(60) {
        return None;
    }
    let centavos = 1 + g.ate(999_999_999_999);
    let texto = valor::texto(centavos);
    Some(if g.chance(50) {
        texto.replace('.', ",")
    } else {
        texto
    })
}

fn txid_gerado(g: &mut Gerador) -> Option<String> {
    if !g.chance(50) {
        return None;
    }
    let n = 1 + usize::try_from(g.ate(27)).unwrap();
    Some(g.texto_de(
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
        n,
    ))
}

fn descricao_gerada(g: &mut Gerador) -> Option<String> {
    if !g.chance(40) {
        return None;
    }
    let n = 1 + usize::try_from(g.ate(12)).unwrap();
    Some(
        (0..n)
            .map(|_| g.escolhe(&["pedido", "Pão", "de", "Queijo", "42", "ref.", "#"]))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

#[test]
fn seiscentos_casos_montam_e_leem_de_volta() {
    let mut g = Gerador(0x9E37_79B9_7F4A_7C15);
    let mut ok = 0;
    let mut recusados = 0;

    for _ in 0..600 {
        let chave = chave_gerada(&mut g);
        let nome = nome_gerado(&mut g, 3);
        let cidade = nome_gerado(&mut g, 2);
        let valor_txt = valor_gerado(&mut g);
        let txid = txid_gerado(&mut g);
        let descricao = descricao_gerada(&mut g);

        let mut montagem = Cobranca::nova(&chave, &nome, &cidade);
        if let Some(v) = &valor_txt {
            montagem = montagem.and_then(|c| c.com_valor(v));
        }
        if let Some(t) = &txid {
            montagem = montagem.and_then(|c| c.com_txid(t));
        }
        if let Some(d) = &descricao {
            montagem = montagem.and_then(|c| c.com_descricao(d));
        }
        let codigo = montagem.and_then(|c| {
            let codigo = c.codigo()?;
            Ok((c, codigo))
        });

        let (c, codigo) = match codigo {
            Ok(par) => par,
            Err(e) => {
                assert!(
                    matches!(
                        e,
                        Erro::Chave(_)
                            | Erro::Valor(_)
                            | Erro::TxId(_)
                            | Erro::Tamanho { .. }
                            | Erro::Obrigatorio(_)
                    ),
                    "erro inesperado: {e:?} para {chave} / {nome} / {cidade}"
                );
                recusados += 1;
                continue;
            }
        };
        ok += 1;

        let l = leitura::ler(&codigo).unwrap_or_else(|e| panic!("não leu {codigo}: {e}"));
        assert_eq!(l.chave, c.chave.valor(), "chave em {codigo}");
        assert_eq!(l.valor, c.valor.map(valor::texto), "valor em {codigo}");
        assert_eq!(l.nome, texto::ascii(&nome), "nome em {codigo}");
        assert_eq!(l.cidade, texto::ascii(&cidade), "cidade em {codigo}");
        assert_eq!(
            l.txid.as_deref(),
            Some(c.txid.as_deref().unwrap_or("***")),
            "txid em {codigo}"
        );
        assert_eq!(l.descricao, c.descricao, "descrição em {codigo}");
        assert_eq!(l.moeda, "986");
        assert_eq!(l.pais, "BR");
        assert_eq!(l.categoria, "0000");
        assert!(
            codigo.len() <= 512,
            "código grande demais: {}",
            codigo.len()
        );
        assert_eq!(
            format!("{:04X}", crc::crc16(&codigo.as_bytes()[..codigo.len() - 4])),
            &codigo[codigo.len() - 4..]
        );
    }

    assert!(ok >= 300, "poucos casos montados: {ok}");
    assert!(recusados >= 20, "poucos casos recusados: {recusados}");
}

#[test]
fn trocar_um_caractere_do_codigo_e_sempre_detectado() {
    let mut g = Gerador(0x0123_4567_89AB_CDEF);
    let mut testados = 0;
    while testados < 200 {
        let chave = chave_gerada(&mut g);
        let nome = nome_gerado(&mut g, 3);
        let cidade = nome_gerado(&mut g, 2);
        let Ok(c) = Cobranca::nova(&chave, &nome, &cidade) else {
            continue;
        };
        let Ok(codigo) = c.codigo() else { continue };
        let corpo_len = codigo.len() - 4;
        let i = usize::try_from(g.ate(u64::try_from(corpo_len).unwrap())).unwrap();
        let mut bytes = codigo.clone().into_bytes();
        bytes[i] = if bytes[i] == b'X' { b'Y' } else { b'X' };
        let alterado = String::from_utf8(bytes).unwrap();
        assert!(
            leitura::ler(&alterado).is_err(),
            "trocar o byte {i} passou em {alterado}"
        );
        testados += 1;
    }
}

#[test]
fn leitura_de_codigo_com_espacos_nas_pontas_funciona() {
    let codigo = Cobranca::nova("pix@exemplo.com.br", "Ana", "Recife")
        .unwrap()
        .codigo()
        .unwrap();
    let com_espacos = format!("\n  {codigo}\t ");
    assert_eq!(
        leitura::ler(&com_espacos).unwrap().chave,
        "pix@exemplo.com.br"
    );
}
