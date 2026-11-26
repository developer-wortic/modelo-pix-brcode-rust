//! O CRC16 do BR Code: CRC16-CCITT-FALSE, com polinômio 0x1021, valor inicial 0xFFFF,
//! sem reflexão de bits e sem XOR final. Vetor de conferência: "123456789" dá 0x29B1.

const POLINOMIO: u16 = 0x1021;
const INICIAL: u16 = 0xFFFF;

/// CRC16-CCITT-FALSE dos bytes dados, feito bit a bit.
#[must_use]
pub fn crc16(dados: &[u8]) -> u16 {
    dados.iter().fold(INICIAL, |crc, &byte| {
        (0..8).fold(crc ^ (u16::from(byte) << 8), |crc, _| {
            if crc & 0x8000 != 0 {
                (crc << 1) ^ POLINOMIO
            } else {
                crc << 1
            }
        })
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn vetor_de_conferencia_do_crc_ccitt_false() {
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }

    #[test]
    fn texto_vazio_devolve_o_valor_inicial() {
        assert_eq!(crc16(b""), 0xFFFF);
    }

    #[test]
    fn um_byte_conhecido() {
        assert_eq!(crc16(b"A"), 0xB915);
    }

    #[test]
    fn qualquer_troca_de_um_byte_muda_o_crc() {
        let original = b"00020126580014br.gov.bcb.pix6304";
        let base = crc16(original);
        for i in 0..original.len() {
            let mut alterado = original.to_vec();
            alterado[i] = alterado[i].wrapping_add(1);
            assert_ne!(crc16(&alterado), base, "posição {i}");
        }
    }
}
