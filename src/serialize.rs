use std::collections::BTreeMap;

use alloy_primitives::hex;
use serde::{Serializer, ser::SerializeSeq};

use crate::{DynSolType, Selector, Slot, StateMutability, control_flow_graph::Block};

pub fn selector<S: Serializer>(val: &Selector, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&hex::encode(val))
}

pub fn arguments<S: Serializer>(
    val: &Option<Vec<DynSolType>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match val {
        Some(args) => {
            let s: String = args
                .iter()
                .map(|t| t.sol_type_name().to_string())
                .collect::<Vec<String>>()
                .join(",");
            serializer.serialize_str(&s)
        }
        None => serializer.serialize_none(),
    }
}

pub fn state_mutability<S: Serializer>(
    val: &Option<StateMutability>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match val {
        Some(sm) => serializer.serialize_str(sm.as_json_str()),
        None => serializer.serialize_none(),
    }
}

pub fn slot<S: Serializer>(val: &Slot, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&hex::encode(val))
}

pub fn vec_selector<S: Serializer>(val: &Vec<Selector>, serializer: S) -> Result<S::Ok, S::Error> {
    let mut s = serializer.serialize_seq(Some(val.len()))?;
    for sel in val {
        s.serialize_element(&hex::encode(sel))?;
    }
    s.end()
}

pub fn blocks<S: Serializer>(
    val: &BTreeMap<usize, Block>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut s = serializer.serialize_seq(Some(val.len()))?;
    for b in val.values() {
        s.serialize_element(b)?;
    }
    s.end()
}
