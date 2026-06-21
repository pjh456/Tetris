use crate::Layer;

#[derive(Debug, Clone, PartialEq)]
pub struct MlpPolicy {
    input_dim: usize,
    output_dim: usize,
    layers: Vec<Layer>,
}

impl MlpPolicy {
    pub fn new(input_dim: usize, output_dim: usize, layers: Vec<Layer>) -> Self {
        Self {
            input_dim,
            output_dim,
            layers,
        }
    }

    pub fn input_dim(&self) -> usize {
        self.input_dim
    }

    pub fn output_dim(&self) -> usize {
        self.output_dim
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    pub fn load_from_slice(bytes: &[u8]) -> Result<Self, crate::InferError> {
        crate::weights::load_from_slice(bytes)
    }

    pub fn load_from_str(input: &str) -> Result<Self, crate::InferError> {
        crate::weights::load_from_str(input)
    }

    pub fn forward(&self, x: &[f32]) -> Vec<f32> {
        let mut activation = x.to_vec();
        for (layer_index, layer) in self.layers.iter().enumerate() {
            let mut next = layer.bias.clone();
            for (output_index, row) in layer.weight.iter().enumerate() {
                let sum = row
                    .iter()
                    .zip(&activation)
                    .fold(layer.bias[output_index], |acc, (weight, value)| {
                        acc + weight * value
                    });
                next[output_index] = sum;
            }

            if layer_index + 1 < self.layers.len() {
                activation = next.into_iter().map(f32::tanh).collect();
            } else {
                activation = next;
            }
        }
        activation
    }

    pub fn act(&self, x: &[f32], mask: &[bool], temperature: f32) -> usize {
        self.act_seeded(x, mask, temperature, 0x5EED_5EED)
    }

    /// Like [`act`](Self::act) but with an explicit sampling seed, so independent
    /// callers (e.g. multiple bots) on identical observations can sample
    /// different actions instead of moving in lockstep.
    pub fn act_seeded(&self, x: &[f32], mask: &[bool], temperature: f32, seed: u64) -> usize {
        let mut logits = self.forward(x);
        for (index, allowed) in mask.iter().copied().enumerate() {
            if !allowed && let Some(logit) = logits.get_mut(index) {
                *logit = f32::NEG_INFINITY;
            }
        }
        softmax_sample_seeded(&logits, temperature, seed)
    }
}

pub fn softmax_sample(logits: &[f32], temperature: f32) -> usize {
    softmax_sample_seeded(logits, temperature, 0x5EED_5EED)
}

pub fn softmax_sample_seeded(logits: &[f32], temperature: f32, seed: u64) -> usize {
    if logits.is_empty() {
        return 0;
    }

    if temperature <= 1.0e-6 {
        return argmax(logits);
    }

    let max_logit = logits
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .fold(f32::NEG_INFINITY, f32::max);
    if !max_logit.is_finite() {
        return 0;
    }

    let mut total = 0.0;
    let mut weights = Vec::with_capacity(logits.len());
    for &logit in logits {
        let weight = if logit.is_finite() {
            ((logit - max_logit) / temperature).exp()
        } else {
            0.0
        };
        total += weight;
        weights.push(weight);
    }

    if total <= 0.0 || !total.is_finite() {
        return argmax(logits);
    }

    let mut threshold = lcg_unit(seed) * total;
    for (index, weight) in weights.into_iter().enumerate() {
        if threshold <= weight {
            return index;
        }
        threshold -= weight;
    }

    argmax(logits)
}

fn argmax(logits: &[f32]) -> usize {
    let mut best_index = 0;
    let mut best_value = f32::NEG_INFINITY;
    for (index, value) in logits.iter().copied().enumerate() {
        if value > best_value {
            best_index = index;
            best_value = value;
        }
    }
    best_index
}

fn lcg_unit(seed: u64) -> f32 {
    let next = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    let bits = (next >> 48) as u16;
    f32::from(bits) / f32::from(u16::MAX)
}
