use super::*;
use crate::structs::*;

#[tokio::test]
async fn test_has_control_over() {
    let c = make_controller(true).await;

    assert!(c.has_control_over(&Actor {
        id: 1,
        login: "test-app[bot]".to_string()
    }));
    assert!(!c.has_control_over(&Actor {
        id: 1,
        login: "test-app".to_string()
    }));
    assert!(!c.has_control_over(&Actor {
        id: 2,
        login: "ppy".to_string()
    }));
}
