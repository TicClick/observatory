use crate::test::GitHubServer;

use super::{Controller, ControllerRequest};

/// Generated with: ssh-keygen -m PEM -t rsa -b 2048
static TEST_APP_PRIVATE_KEY: &str = "
-----BEGIN RSA PRIVATE KEY-----
MIIEowIBAAKCAQEA2ufOyY7mlBGYozXeTKc9FHPOneZgcp+pmt3zE34zf+4BeqgR
6ydcnS1uE+lmqvI3YEBmi7yIixKDXoq2VSZqb4o7X/eDoeqF1lKo/PqfcHZWc54X
VtRdvkdTE2MWlFXRKt9255KagmatMmZVqWBPC/NLU0m9PaYDsog3d02Lb6rXOomP
0IlPH4tb0EdjYYmMg9VVPVjTGqAMXAaNBSgeU8fOMdWxEoaD08p9qQjt8F1ELlc5
+URDuq3FgFDHTTbWqpmar2fzGFgI1Wk7LdbEb/cKT18X7DCBEJLGdoLijKhQjwLL
Aspb1aNdwzFcmYKK60tLB598TBUd3ieiygQkYwIDAQABAoIBACiPUOpZtvFyfTSo
c4MCbbfPaVYqbG5wlO1j+HkBJiuq/s0qPP+0MF3TIBVCZsp/zLDh3d5AVZBnIr4u
t2/5iTkXhL7YTqR+nsPCVxtgmJAu7P/JKAvnl2L9NjBeaL2dVP87nn1z1XsZ6Tdw
bKjQdnUBZFCPVigJDaBTyuspDA/pYLFhvYc6KZHdkO+ShwO3GdApfVjRGlg5IaOF
CPXP8BS5kqLXLQhC71rrvCEcCJ6biyYbTkHT3On1kCnvx/OwfvVgDNVKV6rfYZ5y
QcY0jJR6AFw0WT0WsG6lFdrzLU45iuMbJE7WegPQokvP2ArLipn6xGBn4pw8pdsf
Z+i1onkCgYEA/67DDXpHMYHtHoDStcattftRJnAyQx5UAk3XuV0vWmXL4fcwtB+e
JMzD6H4YSQy7yqKF5UO0laQgkc5/cuz4Y4MuIQwbyIDsg3LCjE1BFHJZNTUNWiXF
fW7Jxr6r2sf3yKfZwMPW3GUt82tgRe921hybGFp/gDDJrE+NFIVTtB8CgYEA2y1c
Ugnc/TY/7B7J28pq49oDbHXG43FVuAhEBnecjfIKBOLtAdoNdnPkTVgi+QkRszEn
slUAr4EcjsI955/hofrk7L0uwfpAe2rKmjUDALFEFt/oLIKZMbB37Xs6bVVbsbHy
zuEOVqOnRbFkRApg7tB49JCTAieUgx+PC1Mupz0CgYA8SJw8pUP77EJYGs+ThFCY
w7SSd4miQZhVIr1mOw5bJf04PewBzCKhUpYuTuyy7ImqcT9YmuoNDjGPrzxlgHHg
JKHPsOcsExmwtHIiWmSpyXw3C1cmlhlGRcTVU0d5wgQuD0VMKeCS/lgjOIHue1Nt
kDkROOUu+FHUir0cxYLCyQKBgH3iQrkX0yZX50TtthCX5OazS+4agz4U1R/bF38D
ahaY4qpFz8yVedAD5ieKLKQOUm0yGVOywK8Mn+NaqwWC7awEF0HlsppU6n44Kt+A
/RWDutDMj2QpKmXArlDmyvsK4Jxh0UyDNKIMYsGDjkwKDfx8HkyRUO4W35SkJpth
jlUdAoGBAJvGgeUvbc5s7fTvnWF8ZH9IMcanSYeHEiob1C6mM4nHjFyqxjv1tTQL
CNzrJcE1tsZ38RAc6HEIieLnDONH195NxUurYS6u6nwIXnzq78xWx/QjzNgPqni0
U0sfVofQ+RD9J5VpyP89BJjcSUHJR8ZDIwYQBzW5AG+z7dXD4Zkn
-----END RSA PRIVATE KEY-----";

async fn make_controller(
    server: &GitHubServer,
    init: bool,
) -> (tokio::sync::mpsc::Sender<ControllerRequest>, Controller) {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let mut c = Controller::new(
        rx,
        server.url.clone(),
        crate::test::TEST_APP_ID.to_string(),
        TEST_APP_PRIVATE_KEY.to_string(),
        crate::config::Controller {
            post_comments: true,
        },
    );
    if init {
        c.init().await.unwrap();
    }
    (tx, c)
}

async fn new_controller(server: &GitHubServer, init: bool) -> Controller {
    make_controller(server, init).await.1
}

mod tests_base;
mod tests_comments;
mod tests_conflicts;
mod tests_installations_repos;
