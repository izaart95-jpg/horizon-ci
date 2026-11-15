//! Java 26.3 compact/full block-state JSON codecs over a pinned registry.
use super::{BlockRegistry, BlockStateId};
use crate::compression::invalid;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
};

/// Property kinds accompany the imported state registry. Integer properties
/// parse Java decimal strings; enum and boolean properties use exact names.
pub struct BlockStateJsonCodec<'a> {
    registry: &'a BlockRegistry,
    integers: BTreeSet<(String, String)>,
    counts: BTreeMap<String, usize>,
}
impl<'a> BlockStateJsonCodec<'a> {
    pub fn new(
        registry: &'a BlockRegistry,
        integers: BTreeSet<(String, String)>,
    ) -> io::Result<Self> {
        for (block, property) in &integers {
            let state = registry
                .default_state(block)
                .and_then(|id| registry.state(id));
            if !state.is_some_and(|state| state.properties.contains_key(property)) {
                return Err(invalid("integer property is absent from block registry"));
            }
        }
        let mut counts = BTreeMap::new();
        for state in &registry.states {
            *counts.entry(state.name.clone()).or_insert(0) += 1;
        }
        Ok(Self {
            registry,
            integers,
            counts,
        })
    }
    /// `full` selects FULL_CODEC, which requires the object representation.
    /// Invalid/missing properties keep their defaults, as vanilla's codec does.
    pub fn decode(&self, json: &str, full: bool) -> io::Result<BlockStateId> {
        let value: Value = serde_json::from_str(json).map_err(|e| invalid(e.to_string()))?;
        let name = if !full && value.is_string() {
            value.as_str()
        } else {
            value.get("id").and_then(Value::as_str)
        }
        .ok_or_else(|| invalid("missing block identifier"))?;
        let (namespace, path) = name.split_once(':').unwrap_or(("minecraft", name));
        let name = format!(
            "{}:{path}",
            if namespace.is_empty() {
                "minecraft"
            } else {
                namespace
            }
        );
        let default = match self.registry.default_state(&name) {
            Some(default) => default,
            // The 1.8 build reads 26.3 world generation, whose files name
            // blocks 1.8 never had. Map the known families to their nearest
            // 1.8 block and leave the rest out as air; a 26.3 build stays
            // strict so a typo in a data pack still fails loudly.
            None if horizon_core::target::is_1_8_9() => {
                let alias = crate::legacy_1_8_9::legacy_alias(&name).unwrap_or("minecraft:air");
                self.registry
                    .default_state(alias)
                    .or_else(|| self.registry.default_state("minecraft:air"))
                    .ok_or_else(|| invalid("unknown block identifier"))?
            }
            None => return Err(invalid("unknown block identifier")),
        };
        let mut state = self.registry.state(default).unwrap().clone();
        if let Some(properties) = value.get("properties").and_then(Value::as_object) {
            for (key, raw) in properties {
                if !state.properties.contains_key(key) {
                    continue;
                }
                let Some(value) = raw.as_str() else {
                    continue;
                };
                let candidate = if self.integers.contains(&(name.clone(), key.clone())) {
                    let Some(value) = java_integer(value) else {
                        continue;
                    };
                    value.to_string()
                } else {
                    value.to_owned()
                };
                let previous = state.properties.insert(key.clone(), candidate).unwrap();
                if self.registry.id(&state).is_none() {
                    state.properties.insert(key.clone(), previous);
                }
            }
        }
        self.registry
            .id(&state)
            .ok_or_else(|| invalid("block state is absent from registry"))
    }
    pub fn encode(&self, id: BlockStateId, full: bool) -> io::Result<String> {
        let state = self
            .registry
            .state(id)
            .ok_or_else(|| invalid("unknown block state ID"))?;
        let value = if !full && self.registry.default_state(&state.name) == Some(id) {
            Value::String(state.name.clone())
        } else if self.counts[&state.name] == 1 {
            serde_json::json!({"id":state.name})
        } else {
            serde_json::json!({"id":state.name,"properties":state.properties})
        };
        serde_json::to_string(&value).map_err(|e| invalid(e.to_string()))
    }
}

pub(crate) use horizon_core::text::parse_java_integer as java_integer;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nbt::{JavaString, Tag};

    fn registry(states: &[&str]) -> BlockRegistry {
        let list: Vec<Tag> = states
            .iter()
            .map(|name| Tag::compound([("id", Tag::String((*name).into()))]))
            .collect();
        let defaults: BTreeMap<JavaString, Tag> = states
            .iter()
            .enumerate()
            .map(|(index, name)| (JavaString::from(*name), Tag::Int(index as i32)))
            .collect();
        BlockRegistry::from_nbt(&Tag::compound([
            ("DataVersion", Tag::Int(crate::DATA_VERSION)),
            ("states", Tag::List(list)),
            ("defaults", Tag::Compound(defaults)),
        ]))
        .unwrap()
    }

    #[test]
    fn current_only_worldgen_blocks_fall_back_on_the_1_8_target() {
        if !horizon_core::target::is_1_8_9() {
            return;
        }
        let registry = registry(&[
            "minecraft:air",
            "minecraft:stone",
            "minecraft:oak_log",
            "minecraft:oak_leaves",
        ]);
        let codec = BlockStateJsonCodec::new(&registry, BTreeSet::new()).unwrap();
        let stone = registry.default_state("minecraft:stone").unwrap();
        let air = registry.default_state("minecraft:air").unwrap();
        assert_eq!(
            codec.decode(r#""minecraft:deepslate""#, false).unwrap(),
            stone
        );
        assert_eq!(
            codec
                .decode(r#""minecraft:cherry_sapling""#, false)
                .unwrap(),
            air
        );
        assert_eq!(
            codec.decode(r#""minecraft:not_a_block""#, false).unwrap(),
            air
        );
    }
}
