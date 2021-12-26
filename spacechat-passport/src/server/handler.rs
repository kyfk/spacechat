use crate::{PrivateKey, PublicKey};
use actix_web::{
    get, post, web,
    HttpRequest, HttpResponse, Responder,
};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::sign::{Signer, Verifier};
use tracing::error;
use uuid::Uuid;
use crate::protos::generated::spacechat_passport::{PostIdAllocateResponse, GetIdVerifyRequest};

#[get("/1/health")]
pub async fn get_health(_req: HttpRequest) -> impl Responder {
    ""
}

#[post("/1/id/allocate")]
pub async fn post_id_allocate(
    keys: web::Data<(PublicKey, PrivateKey)>,
    passphrase: web::Data<Vec<u8>>,
    _req: HttpRequest,
) -> HttpResponse {
    let rsa = match Rsa::private_key_from_pem_passphrase(&keys.1, &passphrase) {
        Ok(rsa) => rsa,
        Err(e) => {
            error!("failed to create pkey from pem with passphrase: {}", e);
            return HttpResponse::InternalServerError().body("{}");
        }
    };

    let pkey = match PKey::from_rsa(rsa) {
        Ok(pkey) => pkey,
        Err(e) => {
            error!("failed to create pkey from pem with passphrase: {}", e);
            return HttpResponse::InternalServerError().body("{}");
        }
    };

    let mut signer = match Signer::new(MessageDigest::sha512(), &pkey) {
        Ok(signer) => signer,
        Err(e) => {
            error!("failed to construct a signer: {}", e);
            return HttpResponse::InternalServerError().body("{}");
        }
    };

    let uuid = Uuid::new_v4().to_string();

    if let Err(e) = signer.update(uuid.as_bytes()) {
        error!("failed signer to update a uuid: {}", e);
        return HttpResponse::InternalServerError().body("{}");
    };

    let signature = match signer.sign_to_vec() {
        Ok(signature) => signature,
        Err(e) => {
            error!("failed to generate a signature: {}", e);
            return HttpResponse::InternalServerError().body("{}");
        }
    };

    HttpResponse::Ok().json(serde_json::to_string(&PostIdAllocateResponse {
        id: uuid,
        signature: base64::encode(signature),
        ..std::default::Default::default()
    }).unwrap())
}

#[get("/1/id/verify")]
pub async fn get_id_verify(
    keys: web::Data<(PublicKey, PrivateKey)>,
    _passphrase: web::Data<Vec<u8>>,
    web::Query(params): web::Query<GetIdVerifyRequest>,
) -> HttpResponse {
    let rsa = match Rsa::public_key_from_pem(&keys.0) {
        Ok(rsa) => rsa,
        Err(e) => {
            error!("failed to construct a Rsa from the public key: {}", e);
            return HttpResponse::InternalServerError().body("");
        }
    };

    let pkey = match PKey::from_rsa(rsa) {
        Ok(pkey) => pkey,
        Err(e) => {
            error!("failed to construct a PKey from the Rsa: {}", e);
            return HttpResponse::InternalServerError().body("");
        }
    };

    let mut verifier = Verifier::new(MessageDigest::sha512(), &pkey).unwrap();
    if let Err(e) = verifier.update(params.id.as_bytes()) {
        error!("failed to feed the id: {}", e);
        return HttpResponse::InternalServerError().body("");
    }

    let bytes = match base64::decode(params.signature) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("failed to decode signature: {}", e);
            return HttpResponse::InternalServerError().body("");
        }
    };

    match verifier.verify(&bytes) {
        Ok(t) => if t {
                HttpResponse::Ok().body("")
            } else {
                HttpResponse::Forbidden().body("")
            },
        Err(e) => {
            error!("failed to verify: {}", e);
            HttpResponse::InternalServerError().body("")
        },
    }
}
