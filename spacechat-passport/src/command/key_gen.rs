use openssl::rsa::Rsa;
use openssl::symm::Cipher;
use std::fs::File;
use std::io::Write;

pub fn key_gen(dist: &str, passphrase: &str) -> Result<(), Box<dyn std::error::Error>> {
    let rsa = Rsa::generate(2048)?;

    let private_key: Vec<u8> =
        rsa.private_key_to_pem_passphrase(Cipher::aes_256_cbc(), passphrase.as_bytes())?;
    let public_key: Vec<u8> = rsa.public_key_to_pem()?;

    let mut private_key_file = File::create(dist)?;
    private_key_file.write_all(&private_key)?;

    let mut public_key_file = File::create(format!("{}.pub", dist))?;
    public_key_file.write_all(&public_key)?;

    Ok(())
}
