use google_sheets4::{
    client::Hub,
    hyper::{client::HttpConnector, Client},
    hyper_rustls::{self, HttpsConnector},
    oauth2::{ServiceAccountAuthenticator, ServiceAccountKey},
    Sheets,
};

use crate::cli::Cli;

const SPREADSHEET_ID: &str = "1o656_BLxhxnU4ovssiZv41BLqhCRT5qMcSVp1hojPfM";

pub async fn get_client(cli: &Cli) -> anyhow::Result<()> {
    println!("Getting sheets client");

    let scopes = vec![
        "https://www.googleapis.com/auth/drive",
        "https://www.googleapis.com/auth/drive.file",
        "https://www.googleapis.com/auth/drive.readonly",
        "https://www.googleapis.com/auth/spreadsheets",
        "https://www.googleapis.com/auth/spreadsheets.readonly",
    ];
    let secret = get_secret();
    let auth = ServiceAccountAuthenticator::builder(secret).build().await?;
    let _ = auth.token(&scopes).await?;
    let hub = Sheets::new(
        Client::builder().build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_or_http()
                .enable_http1()
                .enable_http2()
                .build(),
        ),
        auth,
    );

    let range = format!("Sheet{}!A:A", cli.server_index);
    let index = get_first_empty_row(&hub, &range).await;

    println!("Index for {range} is {:?}", index);
    Ok(())
}

async fn get_first_empty_row(
    hub: &Sheets<HttpsConnector<HttpConnector>>,
    range: &str,
) -> Option<usize> {
    match hub
        .spreadsheets()
        .values_get(SPREADSHEET_ID, &range)
        .doit()
        .await
    {
        Ok((_, value_range)) => {
            if let Some(values) = value_range.values {
                Some(values.len() + 1)
            } else {
                Some(1)
            }
        }
        Err(_) => None,
    }
}

fn get_secret() -> ServiceAccountKey {
    ServiceAccountKey {
        key_type:Some( "service_account".into()),
        private_key_id: Some("92c8db4b9ef724bae2c876e07c166d9bba2eb734".into()),
        client_id: Some("115940951748583473356".into()),
        client_email:"nikola@stellar-utility-379116.iam.gserviceaccount.com".into(),
        token_uri: "https://oauth2.googleapis.com/token".into(),
        auth_uri: Some("https://accounts.google.com/o/oauth2/auth".into()),
        project_id: Some("stellar-utility-379116".into()),
        auth_provider_x509_cert_url: Some("https://www.googleapis.com/oauth2/v1/certs".into()),
        client_x509_cert_url: Some(
            "https://www.googleapis.com/robot/v1/metadata/x509/nikola%40stellar-utility-379116.iam.gserviceaccount.com".into(),
        ),
        private_key: "-----BEGIN PRIVATE KEY-----\nMIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC6m33qIR+XuIUM\nM4MhKkE/lOrE4s9MLcAdHyBVJymA+aKpBOu028LzB+k8oBtAQ6fMMcz6vDbRABUM\nDzcb+ahZ45ji2PkDATIVNY2fDG47lnJbxZ3xg2Wn/P46MSU8WYXHFbqUR6jVFWXp\nBhUJX+O3HBCYK5NbFtABwCY3hbzt/Ncy1znkBiWK9WN8l3LMywwqFkXW9jDSzp06\nCmR/bhImVSQxkgeNhhY32/IwFDm73446VEid/eZ2wlWf8XhHYUbXl9e087QAHvaB\nCVH8tNbTzzBPqJ2FcHW4biXLXyNwUPFY1bbSeLo9jViqkDX+r/WD5IGs7sfYUhB7\n8aXGC9KFAgMBAAECggEABYUE5/B74T59fPtnFQuNa4aJnTJCRHQT+yiJCcvDQAPi\nSlKRcEOR1CN3RCpONAvsQi906zO3AV6ZwMYQcLzlPGdthcQ6NVsLMrJnUn2JIy0T\ni+BgCB1FW/8xO+JpQgw510YuwyUNeuQLpCVgaOsTrr5fRUkArlCR7YNT/g9wI6/q\ndTT6UwPagcYr322H5kenlPTG+7cGecXMpGw/s2iYeROqieA9XL7UT/GbMHMETHIO\nVCIJYgxNzjVpESzHXwMncuiJd6Sie0wuv2EJgRQF7T/TAmMX3sTo/O1uzFuhvc1T\nn2fj/L+nmFwzic2DzJN67+CISuzx8qyF5ODD8SWdQQKBgQDdpoCO99eL9f56F7mM\n5gcQE9Cd9DAjtzaPrra2tdRqY74vmcg+HDC1EDHoDQCDnHnaMbiaG0Pc66otTy4G\nnDDXs6GULGSgGA8YCNIrsNaM5Tp/rteG1bjv+G3pDZduAqf5q3G30LPccE+Y9cM9\ncW+oBwfWiQJjYw/U8oipmNgG5QKBgQDXhrwFL+fg4L5vr9L1tStoGSFH8dPTjlGL\nK7qrhYy6nz65Odsv/Wy/JW4Q/zH8lQRFq/w3UITWCbZUeoPZwqBtSItxZpsULfdu\nijRCeeNOc7smy3V4bfrGBOGRwAjNNULUDmM3xlKnhdK6F9bcrMbe+L03MklhRdpK\nq7mQ44FDIQKBgQDV0ie2w9SFylshgP2YtNcfZV4c4lIGQlo6JbtRavttXqc72EhZ\n0mwSX3sldlWGoU7TdJ+22pKO7jEO4JFwAwEDNOCsxl6UKmF1OB031LJE3WWfgxWb\nl1V++dNdvaTVlW5h5kgfoQ/Bmf7PelZMUb/7Aj1HcoiBRDEjpoy7vxy3GQKBgDx4\nnUCHVHQQGt6TYol2L5uhkWjyPRDamZ6GwnVlnzqte5fU1977KAvpoJw8PfY0iWJT\nAw0yFlNHnlTNmzj6FrES7az/sPtUelwVgtwz/scASb50z5zensH4lKGkU9Pf4cRF\n1SjNCFvgfGOiVLLN926QM+bMwTH9u2XAEOzKKHaBAoGBAMRvAiiqll0W6qRnT4qI\nOw/AG4RKI7srvuQKPzJLdVRKfRYK53udvtd2wqAe8VWCdkocnjn/sl8spgvh5H0l\n+U6L6ap5imyGB1xBhb3JjjGJDbWN+6RP17UBjZc6nnebeWGuX5ClT0qDt/rKdN6b\nwkDVfqoKf9Keno6LMtZXVsST\n-----END PRIVATE KEY-----\n".into(),
    }
}
