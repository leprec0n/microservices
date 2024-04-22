use std::{collections::HashMap, error::Error};

use askama::Template;
use axum::response::Html;
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};

pub mod db;
pub mod request;

mod model;

pub use model::*;
use reqwest::StatusCode;
use tracing::{debug, warn};

use crate::template::Snackbar;

use self::{
    db::{jwt_from_db, store_jwt},
    request::jwt_from_auth_provider,
};

pub async fn get_valid_jwt(
    db_client: &tokio_postgres::Client,
    req_client: &reqwest::Client,
    auth_host: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<JWT, Box<dyn Error>> {
    // Get valid jwt from db
    match jwt_from_db(db_client).await {
        Ok(r) => {
            return Ok(JWT {
                access_token: r.get("access_token"),
                expires_in: r.get("expires"),
                scope: r.get("scope"),
                token_type: r.get("token_type"),
            })
        }
        Err(e) => debug!("Could not get jwt from db: {:?}", e),
    };

    // Get new token from provider
    let response: reqwest::Response =
        jwt_from_auth_provider(req_client, auth_host, client_id, client_secret).await?;

    if response.status() != StatusCode::OK {
        Err("StatusCode not OK")?
    }

    let jwt: JWT = response.json().await?;

    // Store jwt in db
    store_jwt(db_client, &jwt).await;

    Ok(jwt)
}

fn create_certificate(cert_body: &str) -> String {
    format!("-----BEGIN CERTIFICATE-----\n{cert_body}\n-----END CERTIFICATE-----")
}

pub fn extract_id_token(
    params: HashMap<String, String>,
    snackbar: &mut Snackbar<'_>,
    auth_keys: &Keys,
    client_aud: &str,
) -> Result<Claims, (StatusCode, Html<String>)> {
    // Get id token param
    let id_token: &String = match params.get("id_token") {
        Some(v) => v,
        None => {
            snackbar.message = "No parameter id_token";
            return Err((StatusCode::BAD_REQUEST, Html(snackbar.render().unwrap())));
        }
    };

    // Decode token
    match decode_token(
        create_certificate(&auth_keys.keys[0].x5c[0]), // Might not work if certificate is in different position of key
        client_aud,
        id_token,
    ) {
        Ok(v) => Ok(v.claims),
        Err(e) => {
            warn!("Cannot decode id token: {:?}", e);
            snackbar.message = "Could not process request";
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(snackbar.render().unwrap()),
            ));
        }
    }
}

fn decode_token(
    cert: String,
    client_aud: &str,
    token: &str,
) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    // Create key from pem
    let key: DecodingKey = DecodingKey::from_rsa_pem(cert.as_bytes())?;

    // Validation params
    let mut validation: Validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[client_aud]);

    // Decode token
    jsonwebtoken::decode::<Claims>(token, &key, &validation)
}

#[cfg(test)]
mod test {
    use jsonwebtoken::{EncodingKey, Header};

    use super::*;

    #[test]
    fn test_create_certificate() {
        assert_eq!(3, create_certificate("cert_body").lines().count());
    }

    #[test]
    fn test_decode_token() {
        let header: Header = Header::new(Algorithm::RS256);
        let claims: Claims = Claims {
            aud: "aud".to_owned(),
            email: "test@gmail.com".to_owned(),
            email_verified: true,
            exp: 9999999999,
            iat: 1516239022,
            iss: "me".to_owned(),
            name: "test".to_owned(),
            nickname: "tester".to_owned(),
            picture: "none".to_owned(),
            sid: "sid".to_owned(),
            sub: "1234567890".to_owned(),
            updated_at: "now".to_owned(),
        };

        let private_key: &str = "-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQC7VJTUt9Us8cKj
        MzEfYyjiWA4R4/M2bS1GB4t7NXp98C3SC6dVMvDuictGeurT8jNbvJZHtCSuYEvu
        NMoSfm76oqFvAp8Gy0iz5sxjZmSnXyCdPEovGhLa0VzMaQ8s+CLOyS56YyCFGeJZ
        qgtzJ6GR3eqoYSW9b9UMvkBpZODSctWSNGj3P7jRFDO5VoTwCQAWbFnOjDfH5Ulg
        p2PKSQnSJP3AJLQNFNe7br1XbrhV//eO+t51mIpGSDCUv3E0DDFcWDTH9cXDTTlR
        ZVEiR2BwpZOOkE/Z0/BVnhZYL71oZV34bKfWjQIt6V/isSMahdsAASACp4ZTGtwi
        VuNd9tybAgMBAAECggEBAKTmjaS6tkK8BlPXClTQ2vpz/N6uxDeS35mXpqasqskV
        laAidgg/sWqpjXDbXr93otIMLlWsM+X0CqMDgSXKejLS2jx4GDjI1ZTXg++0AMJ8
        sJ74pWzVDOfmCEQ/7wXs3+cbnXhKriO8Z036q92Qc1+N87SI38nkGa0ABH9CN83H
        mQqt4fB7UdHzuIRe/me2PGhIq5ZBzj6h3BpoPGzEP+x3l9YmK8t/1cN0pqI+dQwY
        dgfGjackLu/2qH80MCF7IyQaseZUOJyKrCLtSD/Iixv/hzDEUPfOCjFDgTpzf3cw
        ta8+oE4wHCo1iI1/4TlPkwmXx4qSXtmw4aQPz7IDQvECgYEA8KNThCO2gsC2I9PQ
        DM/8Cw0O983WCDY+oi+7JPiNAJwv5DYBqEZB1QYdj06YD16XlC/HAZMsMku1na2T
        N0driwenQQWzoev3g2S7gRDoS/FCJSI3jJ+kjgtaA7Qmzlgk1TxODN+G1H91HW7t
        0l7VnL27IWyYo2qRRK3jzxqUiPUCgYEAx0oQs2reBQGMVZnApD1jeq7n4MvNLcPv
        t8b/eU9iUv6Y4Mj0Suo/AU8lYZXm8ubbqAlwz2VSVunD2tOplHyMUrtCtObAfVDU
        AhCndKaA9gApgfb3xw1IKbuQ1u4IF1FJl3VtumfQn//LiH1B3rXhcdyo3/vIttEk
        48RakUKClU8CgYEAzV7W3COOlDDcQd935DdtKBFRAPRPAlspQUnzMi5eSHMD/ISL
        DY5IiQHbIH83D4bvXq0X7qQoSBSNP7Dvv3HYuqMhf0DaegrlBuJllFVVq9qPVRnK
        xt1Il2HgxOBvbhOT+9in1BzA+YJ99UzC85O0Qz06A+CmtHEy4aZ2kj5hHjECgYEA
        mNS4+A8Fkss8Js1RieK2LniBxMgmYml3pfVLKGnzmng7H2+cwPLhPIzIuwytXywh
        2bzbsYEfYx3EoEVgMEpPhoarQnYPukrJO4gwE2o5Te6T5mJSZGlQJQj9q4ZB2Dfz
        et6INsK0oG8XVGXSpQvQh3RUYekCZQkBBFcpqWpbIEsCgYAnM3DQf3FJoSnXaMhr
        VBIovic5l0xFkEHskAjFTevO86Fsz1C2aSeRKSqGFoOQ0tmJzBEs1R6KqnHInicD
        TQrKhArgLXX4v3CddjfTRJkFWDbE/CkvKZNOrcf1nhaGCPspRJj2KUkj1Fhl9Cnc
        dn/RsYEONbwQSjIfMPkvxF+8HQ==\n-----END PRIVATE KEY-----";

        let key: EncodingKey = EncodingKey::from_rsa_pem(private_key.as_bytes()).unwrap();
        let token: String = jsonwebtoken::encode(&header, &claims, &key).unwrap();

        let public_key = "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAu1SU1LfVLPHCozMxH2Mo
        4lgOEePzNm0tRgeLezV6ffAt0gunVTLw7onLRnrq0/IzW7yWR7QkrmBL7jTKEn5u
        +qKhbwKfBstIs+bMY2Zkp18gnTxKLxoS2tFczGkPLPgizskuemMghRniWaoLcyeh
        kd3qqGElvW/VDL5AaWTg0nLVkjRo9z+40RQzuVaE8AkAFmxZzow3x+VJYKdjykkJ
        0iT9wCS0DRTXu269V264Vf/3jvredZiKRkgwlL9xNAwxXFg0x/XFw005UWVRIkdg
        cKWTjpBP2dPwVZ4WWC+9aGVd+Gyn1o0CLelf4rEjGoXbAAEgAqeGUxrcIlbjXfbc
        mwIDAQAB";
        let cert: String = create_certificate(&public_key);
        let decoded: TokenData<Claims> = decode_token(cert, "aud", &token).unwrap();

        assert_eq!(claims.email, decoded.claims.email);
    }
}
