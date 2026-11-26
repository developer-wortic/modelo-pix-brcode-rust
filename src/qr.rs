//! O QR Code do código Pix em SVG. O codificador é o da crate `qrcodegen` (Rust puro, MIT, sem C).

use std::fmt::Write;

use qrcodegen::{QrCode, QrCodeEcc};

use crate::erro::Erro;

/// Margem em módulos ao redor do QR, como pede a leitura de câmera.
const BORDA: i32 = 4;

/// Gera o QR Code do texto como SVG, com nível de correção M (15% de recuperação).
///
/// # Errors
///
/// Devolve `Erro::Qr` se o texto for grande demais para um QR Code.
pub fn svg(conteudo: &str) -> Result<String, Erro> {
    let codigo = QrCode::encode_text(conteudo, QrCodeEcc::Medium).map_err(|_| Erro::Qr)?;
    let n = codigo.size();
    let total = n + 2 * BORDA;
    let mut caminho = String::new();
    for y in 0..n {
        for x in 0..n {
            if codigo.get_module(x, y) {
                let _ = write!(caminho, "M{} {}h1v1h-1z", x + BORDA, y + BORDA);
            }
        }
    }
    Ok(format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {total} {total}" role="img" aria-label="QR Code do Pix"><rect width="{total}" height="{total}" fill="#ffffff"/><path fill="#1d1d1f" d="{caminho}"/></svg>"##
    ))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn gera_um_svg_com_a_area_certa() {
        let svg = svg("00020101021126580014br.gov.bcb.pix").unwrap();
        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("role=\"img\""));
        assert!(svg.ends_with("</svg>"));
        let codigo =
            QrCode::encode_text("00020101021126580014br.gov.bcb.pix", QrCodeEcc::Medium).unwrap();
        let total = codigo.size() + 2 * BORDA;
        assert!(svg.contains(&format!("viewBox=\"0 0 {total} {total}\"")));
        let modulos = svg.matches('M').count();
        let escuros = (0..codigo.size())
            .flat_map(|y| (0..codigo.size()).map(move |x| (x, y)))
            .filter(|&(x, y)| codigo.get_module(x, y))
            .count();
        assert_eq!(modulos, escuros);
    }

    #[test]
    fn texto_grande_demais_para_o_qr_vira_erro() {
        assert_eq!(svg(&"x".repeat(4000)), Err(Erro::Qr));
    }
}
