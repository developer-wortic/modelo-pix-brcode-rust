//! Chaves Pix: casos válidos e inválidos de cada tipo, e o CNPJ alfanumérico de 2026.

use pix_brcode::chave::{Chave, Tipo, cnpj_valido, cpf_valido};

#[test]
fn cpf_validos_e_invalidos_com_e_sem_mascara() {
    for cpf in [
        "123.456.789-09",
        "12345678909",
        "529.982.247-25",
        "52998224725",
    ] {
        let chave = Chave::interpretar(cpf).unwrap_or_else(|e| panic!("{cpf}: {e}"));
        assert_eq!(chave.tipo(), Tipo::Cpf, "{cpf}");
    }
    for cpf in [
        "123.456.789-00",
        "111.111.111-11",
        "000.000.000-00",
        "123.456.789",
        "1234567890912",
    ] {
        assert!(Chave::interpretar(cpf).is_err(), "devia recusar {cpf}");
    }
}

#[test]
fn cpf_gerado_com_digito_independente_passa() {
    // dígitos verificadores calculados aqui, à parte da biblioteca
    let mut ok = 0;
    for base in 100_000_000u64..100_000_000 + 400 {
        let d: Vec<u32> = base
            .to_string()
            .bytes()
            .map(|b| u32::from(b - b'0'))
            .collect();
        let dv = |n: &[u32], pesos: &[u32]| {
            let s: u32 = n.iter().zip(pesos).map(|(a, b)| a * b).sum();
            let r = (s * 10) % 11;
            if r == 10 { 0 } else { r }
        };
        let v1 = dv(&d, &[10, 9, 8, 7, 6, 5, 4, 3, 2]);
        let mut com1 = d.clone();
        com1.push(v1);
        let v2 = dv(&com1, &[11, 10, 9, 8, 7, 6, 5, 4, 3, 2]);
        let cpf: String = com1
            .iter()
            .chain([v2].iter())
            .map(|x| char::from_digit(*x, 10).unwrap())
            .collect();
        assert!(cpf_valido(&cpf), "CPF gerado {cpf} devia passar");
        assert!(Chave::interpretar(&cpf).is_ok());
        ok += 1;
    }
    assert_eq!(ok, 400);
}

#[test]
fn cnpj_numerico_e_o_alfanumerico_de_2026() {
    // exemplo público da Receita para o alfanumérico (12.ABC.345/01DE-35)
    let c = Chave::interpretar("12.ABC.345/01DE-35").unwrap();
    assert_eq!(c.tipo(), Tipo::Cnpj);
    assert_eq!(c.valor(), "12ABC34501DE35");
    assert!(cnpj_valido("ABCDEFGHIJKL80"));
    assert!(cnpj_valido("11222333000181"));
    assert!(Chave::interpretar("12ABC34501DE34").is_err());
    assert!(Chave::interpretar("ABCDEFGHIJKL81").is_err());
}

#[test]
fn email_com_maiusculas_vira_minusculas_e_formas_ruins_nao_passam() {
    assert_eq!(
        Chave::interpretar("Vendas.Loja@Exemplo.COM")
            .unwrap()
            .valor(),
        "vendas.loja@exemplo.com"
    );
    for ruim in [
        "a@b",
        "a@@b.com",
        "@b.com",
        "a b@c.com",
        "a@c..com",
        "a@.com",
        "a@c.c",
    ] {
        assert!(Chave::interpretar(ruim).is_err(), "devia recusar {ruim}");
    }
}

#[test]
fn telefone_precisa_de_mais_55() {
    let t = Chave::interpretar("+55 (11) 91234-5678").unwrap();
    assert_eq!(t.tipo(), Tipo::Telefone);
    assert_eq!(t.valor(), "+5511912345678");
    assert_eq!(
        Chave::interpretar("+551133334444").unwrap().tipo(),
        Tipo::Telefone
    );
    for ruim in [
        "11912345678",
        "+5511812345678",
        "+5501912345678",
        "+1 555 123 4567",
    ] {
        assert!(Chave::interpretar(ruim).is_err(), "devia recusar {ruim}");
    }
}

#[test]
fn aleatoria_uuid_v4_em_maiusculas_ou_minusculas() {
    let a = Chave::interpretar("3F2504E0-4F89-41D3-9A0C-0305E82C3301").unwrap();
    assert_eq!(a.tipo(), Tipo::Aleatoria);
    assert_eq!(a.valor(), "3f2504e0-4f89-41d3-9a0c-0305e82c3301");
    assert!(Chave::interpretar("3f2504e0-4f89-11d3-9a0c-0305e82c3301").is_err());
    assert!(Chave::interpretar("3f2504e0-4f89-41d3-7a0c-0305e82c3301").is_err());
}

#[test]
fn texto_sem_forma_de_chave_e_recusado_com_mensagem() {
    let e = Chave::interpretar("nada aqui").unwrap_err();
    assert!(e.to_string().contains("não parece CPF"));
}
