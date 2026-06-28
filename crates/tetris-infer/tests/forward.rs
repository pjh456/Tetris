use tetris_infer::{Layer, MlpPolicy, softmax_sample_seeded};

fn policy() -> MlpPolicy {
    MlpPolicy::new(
        2,
        2,
        vec![
            Layer {
                weight: vec![vec![1.0, 1.0], vec![1.0, -1.0]],
                bias: vec![0.0, 0.0],
                norm: None,
                residual: false,
            },
            Layer {
                weight: vec![vec![2.0, 1.0], vec![-1.0, 3.0]],
                bias: vec![0.5, -0.25],
                norm: None,
                residual: false,
            },
        ],
    )
}

#[test]
fn forward_returns_expected_logits() {
    let policy = policy();
    let logits = policy.forward(&[0.5, -0.25]);
    let hidden_0 = 0.25_f32.tanh();
    let hidden_1 = 0.75_f32.tanh();
    let expected_0 = 0.5 + 2.0 * hidden_0 + hidden_1;
    let expected_1 = -0.25 - hidden_0 + 3.0 * hidden_1;

    assert_eq!(logits.len(), 2);
    assert!((logits[0] - expected_0).abs() < 1.0e-6);
    assert!((logits[1] - expected_1).abs() < 1.0e-6);
}

#[test]
fn act_honors_mask() {
    let policy = policy();
    let action = policy.act(&[0.5, -0.25], &[false, true], 0.0);
    assert_eq!(action, 1);
}

#[test]
fn low_temperature_returns_masked_argmax() {
    let policy = policy();
    let action = policy.act(&[0.5, -0.25], &[true, true], 0.0);
    assert_eq!(action, 0);
}

#[test]
fn high_temperature_uses_seeded_distribution() {
    let first = softmax_sample_seeded(&[0.0, 0.0, 0.0, 0.0], 10.0, 1);
    let second = softmax_sample_seeded(&[0.0, 0.0, 0.0, 0.0], 10.0, 1);
    assert_eq!(first, second);
    assert!(first < 4);
}
