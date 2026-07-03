//! Применение RerunAction к KpiContract (P1.4). Меняет только контракт —
//! граф и snapshot.hash остаются прежними.

use contracts::{Constraint, KpiContract, Op, RerunAction, RerunKind};

pub fn apply(contract: &mut KpiContract, action: &RerunAction) {
    let p = &action.payload;
    match action.kind {
        RerunKind::ExcludeFactor => {
            if let Some(f) = &p.factor_id {
                if !contract.excluded_factors.contains(f) {
                    contract.excluded_factors.push(f.clone());
                }
            }
        }
        RerunKind::ChangeWeight => {
            if let (Some(dim), Some(v)) = (&p.dimension, p.value) {
                contract.weights_override.insert(dim.clone(), v);
            }
        }
        RerunKind::AddConstraint => {
            if let (Some(metric), Some(value)) = (&p.metric, p.value) {
                let op = p.op.unwrap_or(Op::LessEqual);
                upsert_constraint(contract, metric, op, value);
            }
        }
        RerunKind::RelaxConstraint => {
            if let (Some(metric), Some(value)) = (&p.metric, p.value) {
                let op = p.op.unwrap_or(Op::LessEqual);
                upsert_constraint(contract, metric, op, value);
            }
        }
        RerunKind::ChangePrice => {
            if let (Some(element), Some(usd_per_t)) = (&p.element, p.usd_per_t) {
                contract.prices_usd_per_t.insert(element.clone(), usd_per_t);
            }
        }
    }
}

fn upsert_constraint(contract: &mut KpiContract, metric: &str, op: Op, value: f64) {
    if let Some(c) = contract.constraints.iter_mut().find(|c| c.metric == metric) {
        c.op = op;
        c.value = value;
    } else {
        contract.constraints.push(Constraint {
            metric: metric.to_string(),
            op,
            value,
            unit: None,
        });
    }
}
