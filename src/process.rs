//! The engineering process as data: document kinds and lifecycle profiles,
//! embedded from `process/` at compile time.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const KINDS: &str = include_str!("../process/kinds.toml");
const PROFILES: [&str; 4] = [
    include_str!("../process/profiles/product.toml"),
    include_str!("../process/profiles/software.toml"),
    include_str!("../process/profiles/re.toml"),
    include_str!("../process/profiles/exercise.toml"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Naming {
    Single,
    Numbered,
    Gate,
    Date,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Kind {
    pub key: String,
    pub code: String,
    pub title: String,
    #[serde(rename = "type")]
    pub doc_type: String,
    pub basis: String,
    pub purpose: String,
    pub naming: Naming,
    pub path: String,
    #[serde(default)]
    pub procedure: bool,
    #[serde(default = "engineering")]
    pub audience: String,
}

fn engineering() -> String {
    "engineering".to_string()
}

impl Kind {
    pub fn end_user(&self) -> bool {
        self.audience == "end-user"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DocState {
    Exists,
    Released,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Requirement {
    pub kind: String,
    pub state: DocState,
    pub min_tier: Option<String>,
}

/// A document a gate requires: the requirements of its phase and of every
/// phase before it, merged per kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateRequirement {
    pub kind: String,
    /// The strongest state any of those phases asks for.
    pub state: DocState,
    /// Set only when the requirements of every tier are listed.
    pub min_tier: Option<String>,
    /// The phase that first requires the document in that state.
    pub phase: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Criterion {
    Text(String),
    Tiered { text: String, min_tier: String },
}

impl Criterion {
    pub fn text(&self) -> &str {
        match self {
            Criterion::Text(text) | Criterion::Tiered { text, .. } => text,
        }
    }

    pub fn min_tier(&self) -> Option<&str> {
        match self {
            Criterion::Text(_) => None,
            Criterion::Tiered { min_tier, .. } => Some(min_tier),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Phase {
    pub id: String,
    pub name: String,
    pub gate: Option<String>,
    pub gate_title: Option<String>,
    pub question: Option<String>,
    pub equivalent: Option<String>,
    pub min_tier: Option<String>,
    #[serde(default)]
    pub require: Vec<Requirement>,
    #[serde(default)]
    pub checks: Vec<String>,
    #[serde(default)]
    pub criteria: Vec<Criterion>,
}

impl Phase {
    pub fn label(&self) -> String {
        format!("{} {}", self.id, self.name)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Profile {
    pub kind: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub tiers: Vec<String>,
    #[serde(rename = "phase")]
    pub phases: Vec<Phase>,
}

impl Profile {
    /// Whether something marked `min_tier` applies to a project at `tier`.
    /// Profiles without tiers apply everything.
    pub fn applies(&self, tier: Option<&str>, min_tier: Option<&str>) -> bool {
        let Some(min) = min_tier else { return true };
        if self.tiers.is_empty() {
            return true;
        }
        let rank = |t: &str| self.tiers.iter().position(|x| x == t);
        match (tier.and_then(rank), rank(min)) {
            (Some(have), Some(need)) => have >= need,
            _ => false,
        }
    }

    pub fn phases_for(&self, tier: Option<&str>) -> Vec<&Phase> {
        self.phases
            .iter()
            .filter(|p| self.applies(tier, p.min_tier.as_deref()))
            .collect()
    }

    pub fn phase(&self, id: &str) -> Option<&Phase> {
        self.phases.iter().find(|p| p.id.eq_ignore_ascii_case(id))
    }

    pub fn phase_by_gate(&self, gate: &str) -> Option<&Phase> {
        self.phases.iter().find(|p| {
            p.gate
                .as_deref()
                .is_some_and(|g| g.eq_ignore_ascii_case(gate))
        })
    }

    pub fn next_phase(&self, id: &str, tier: Option<&str>) -> Option<&Phase> {
        let phases = self.phases_for(tier);
        let at = phases.iter().position(|p| p.id == id)?;
        phases.get(at + 1).copied()
    }

    pub fn requirements<'a>(&self, phase: &'a Phase, tier: Option<&str>) -> Vec<&'a Requirement> {
        phase
            .require
            .iter()
            .filter(|r| self.applies(tier, r.min_tier.as_deref()))
            .collect()
    }

    /// The phases from the first to `phase`. With a tier, the earlier phases
    /// above it are left out; without one, every phase is listed.
    pub fn phases_through(&self, phase: &Phase, tier: Option<&str>) -> Vec<&Phase> {
        let end = self
            .phases
            .iter()
            .position(|p| p.id == phase.id)
            .map_or(self.phases.len(), |at| at + 1);
        self.phases[..end]
            .iter()
            .filter(|p| {
                tier.is_none() || p.id == phase.id || self.applies(tier, p.min_tier.as_deref())
            })
            .collect()
    }

    /// A tier's place in the profile, with 0 for no tier.
    fn tier_rank(&self, tier: Option<&str>) -> usize {
        tier.and_then(|t| self.tiers.iter().position(|x| x == t))
            .map_or(0, |at| at + 1)
    }

    /// The documents a gate requires. Gates are cumulative: a gate requires
    /// the documents of its phase and of every phase before it, each kind once
    /// and in the strongest state any of those phases asks for. With a tier,
    /// only what applies at it; without one, the requirements of every tier,
    /// each with the tier it applies from.
    pub fn gate_requirements(&self, phase: &Phase, tier: Option<&str>) -> Vec<GateRequirement> {
        let mut out: Vec<GateRequirement> = Vec::new();
        for p in self.phases_through(phase, tier) {
            for req in &p.require {
                // A requirement applies from the higher of its phase's tier and
                // its own. Asked about directly, a gate above the project's
                // tier still lists what its own phase requires.
                let own_phase = tier.is_some() && p.id == phase.id;
                let phase_tier = p.min_tier.as_deref().filter(|_| !own_phase);
                let from = [phase_tier, req.min_tier.as_deref()]
                    .into_iter()
                    .max_by_key(|t| self.tier_rank(*t))
                    .flatten();
                if tier.is_some() && !self.applies(tier, from) {
                    continue;
                }
                let min_tier = match tier {
                    Some(_) => None,
                    None => from.map(str::to_string),
                };
                match out
                    .iter_mut()
                    .find(|g| g.kind == req.kind && g.min_tier == min_tier)
                {
                    Some(g) if req.state > g.state => {
                        g.state = req.state;
                        g.phase = p.id.clone();
                    }
                    Some(_) => {}
                    None => out.push(GateRequirement {
                        kind: req.kind.clone(),
                        state: req.state,
                        min_tier,
                        phase: p.id.clone(),
                    }),
                }
            }
        }
        // Listed for every tier, a kind can appear once per tier: drop an
        // entry that a lower tier already requires in the same state or a stronger one.
        let all = out.clone();
        out.retain(|g| {
            !all.iter().any(|o| {
                o.kind == g.kind
                    && o.state >= g.state
                    && self.tier_rank(o.min_tier.as_deref()) < self.tier_rank(g.min_tier.as_deref())
            })
        });
        out
    }

    /// The automated checks of a gate: those of its phase and of every phase
    /// before it, each once.
    pub fn gate_checks(&self, phase: &Phase, tier: Option<&str>) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for p in self.phases_through(phase, tier) {
            for check in &p.checks {
                if !out.contains(&check.as_str()) {
                    out.push(check);
                }
            }
        }
        out
    }

    pub fn criteria<'a>(&self, phase: &'a Phase, tier: Option<&str>) -> Vec<&'a str> {
        phase
            .criteria
            .iter()
            .filter(|c| self.applies(tier, c.min_tier()))
            .map(Criterion::text)
            .collect()
    }

    /// The gate at which a document kind is first baselined (required as released).
    pub fn baseline_gate(&self, kind: &str, tier: Option<&str>) -> Option<&str> {
        self.phases_for(tier).into_iter().find_map(|phase| {
            let released = self
                .requirements(phase, tier)
                .iter()
                .any(|r| r.kind == kind && r.state == DocState::Released);
            if released {
                phase.gate.as_deref()
            } else {
                None
            }
        })
    }
}

pub struct Process {
    pub kinds: Vec<Kind>,
    pub profiles: Vec<Profile>,
}

#[derive(Deserialize)]
struct KindsFile {
    kind: Vec<Kind>,
}

impl Process {
    /// The embedded process. Its consistency is enforced by the tests in this module.
    pub fn get() -> &'static Process {
        static PROCESS: OnceLock<Process> = OnceLock::new();
        PROCESS.get_or_init(|| {
            let kinds = toml::from_str::<KindsFile>(KINDS)
                .expect("process/kinds.toml is valid")
                .kind;
            let profiles = PROFILES
                .iter()
                .map(|text| toml::from_str::<Profile>(text).expect("process profile is valid"))
                .collect();
            Process { kinds, profiles }
        })
    }

    /// A kind by key (`srs`) or code (`SRS`).
    pub fn kind(&self, name: &str) -> Option<&Kind> {
        self.kinds
            .iter()
            .find(|k| k.key.eq_ignore_ascii_case(name) || k.code.eq_ignore_ascii_case(name))
    }

    pub fn profile(&self, kind: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.kind == kind)
    }

    pub fn profile_names(&self) -> Vec<&str> {
        self.profiles.iter().map(|p| p.kind.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_have_unique_keys_and_codes() {
        let process = Process::get();
        for (i, a) in process.kinds.iter().enumerate() {
            for b in &process.kinds[i + 1..] {
                assert_ne!(a.key, b.key);
                assert_ne!(a.code, b.code);
            }
        }
    }

    #[test]
    fn profiles_only_reference_known_kinds_and_tiers() {
        let process = Process::get();
        for profile in &process.profiles {
            for phase in &profile.phases {
                for req in &phase.require {
                    assert!(
                        process.kind(&req.kind).is_some(),
                        "{}: unknown kind {}",
                        profile.kind,
                        req.kind
                    );
                    if let Some(tier) = &req.min_tier {
                        assert!(
                            profile.tiers.contains(tier),
                            "{}: unknown tier {tier}",
                            profile.kind
                        );
                    }
                }
                for criterion in &phase.criteria {
                    if let Some(tier) = criterion.min_tier() {
                        assert!(
                            profile.tiers.iter().any(|t| t == tier),
                            "{}: unknown tier {tier}",
                            profile.kind
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn profiles_only_name_checks_that_exist() {
        for profile in &Process::get().profiles {
            for phase in &profile.phases {
                for check in &phase.checks {
                    assert!(
                        ["requirements-well-formed", "rtm-planned", "rtm-verified"]
                            .contains(&check.as_str()),
                        "{}: unknown check {check}",
                        profile.kind
                    );
                }
            }
        }
    }

    #[test]
    fn no_criterion_repeats_an_automated_check() {
        for profile in &Process::get().profiles {
            for phase in &profile.phases {
                if phase.checks.iter().any(|c| c == "rtm-planned") {
                    assert!(
                        !phase
                            .criteria
                            .iter()
                            .any(|c| c.text().contains("planned test case")),
                        "{} {}: the check already covers planned test cases",
                        profile.kind,
                        phase.id
                    );
                }
            }
        }
    }

    #[test]
    fn gates_and_kind_codes_do_not_collide() {
        let process = Process::get();
        for profile in &process.profiles {
            for gate in profile.phases.iter().filter_map(|p| p.gate.as_deref()) {
                assert!(
                    process.kind(gate).is_none(),
                    "{}: gate {gate} is also a document kind",
                    profile.kind
                );
            }
        }
    }

    #[test]
    fn gates_are_unique_within_a_profile() {
        for profile in &Process::get().profiles {
            let gates: Vec<_> = profile
                .phases
                .iter()
                .filter_map(|p| p.gate.as_deref())
                .collect();
            for (i, g) in gates.iter().enumerate() {
                assert!(
                    !gates[i + 1..].contains(g),
                    "{}: duplicate gate {g}",
                    profile.kind
                );
            }
        }
    }

    #[test]
    fn tiers_filter_phases_and_requirements() {
        let product = Process::get().profile("product").unwrap();
        let desk: Vec<_> = product
            .phases_for(Some("desk"))
            .iter()
            .map(|p| p.id.clone())
            .collect();
        let batch: Vec<_> = product
            .phases_for(Some("batch"))
            .iter()
            .map(|p| p.id.clone())
            .collect();
        assert!(!desk.contains(&"P6".to_string()));
        assert!(batch.contains(&"P6".to_string()));

        let pdr = product.phase_by_gate("PDR").unwrap();
        let desk_kinds: Vec<_> = product
            .requirements(pdr, Some("desk"))
            .iter()
            .map(|r| r.kind.as_str())
            .collect();
        assert!(!desk_kinds.contains(&"icd"));
        let retail_kinds: Vec<_> = product
            .requirements(pdr, Some("retail"))
            .iter()
            .map(|r| r.kind.as_str())
            .collect();
        assert!(retail_kinds.contains(&"icd"));
    }

    fn required(profile: &Profile, gate: &str, tier: Option<&str>) -> Vec<(String, DocState)> {
        profile
            .gate_requirements(profile.phase_by_gate(gate).unwrap(), tier)
            .into_iter()
            .map(|g| (g.kind, g.state))
            .collect()
    }

    #[test]
    fn a_gate_requires_the_documents_of_every_phase_up_to_it() {
        use DocState::{Exists, Released};
        let software = Process::get().profile("software").unwrap();
        let kinds = |list: &[(&str, DocState)]| -> Vec<(String, DocState)> {
            list.iter().map(|(k, s)| (k.to_string(), *s)).collect()
        };
        assert_eq!(
            required(software, "TRR", None),
            kinds(&[
                ("pln", Released),
                ("con", Released),
                ("rsk", Exists),
                ("srs", Released),
                ("vvp", Released),
                ("arc", Released),
            ])
        );
        // The V&V plan only has to exist until Build asks for its release.
        let trr = software.phase_by_gate("TRR").unwrap();
        let vvp = software
            .gate_requirements(trr, None)
            .into_iter()
            .find(|g| g.kind == "vvp")
            .unwrap();
        assert_eq!(vvp.phase, "P3");
        assert_eq!(
            required(software, "DR", None)[4],
            ("vvp".to_string(), Exists)
        );
        // A gate that requires nothing of its own still carries the earlier phases.
        assert!(required(software, "CLOSE", None).contains(&("rel".to_string(), Released)));
    }

    #[test]
    fn every_gate_requires_what_the_gates_before_it_require() {
        for profile in &Process::get().profiles {
            let tiers: Vec<Option<&str>> = if profile.tiers.is_empty() {
                vec![None]
            } else {
                profile.tiers.iter().map(|t| Some(t.as_str())).collect()
            };
            for tier in tiers {
                let phases = profile.phases_for(tier);
                for pair in phases.windows(2) {
                    let later = profile.gate_requirements(pair[1], tier);
                    for earlier in profile.gate_requirements(pair[0], tier) {
                        assert!(
                            later
                                .iter()
                                .any(|g| g.kind == earlier.kind && g.state >= earlier.state),
                            "{} {}: {} is dropped or weakened",
                            profile.kind,
                            pair[1].id,
                            earlier.kind
                        );
                    }
                    for (i, g) in later.iter().enumerate() {
                        assert!(
                            later[i + 1..].iter().all(|o| o.kind != g.kind),
                            "{} {}: {} is listed twice",
                            profile.kind,
                            pair[1].id,
                            g.kind
                        );
                    }
                    let checks = profile.gate_checks(pair[1], tier);
                    for check in profile.gate_checks(pair[0], tier) {
                        assert!(checks.contains(&check));
                    }
                }
            }
        }
    }

    #[test]
    fn a_tier_leaves_out_what_applies_only_above_it() {
        use DocState::Released;
        let product = Process::get().profile("product").unwrap();
        let has = |gate: &str, tier: Option<&str>, kind: &str| {
            required(product, gate, tier)
                .iter()
                .any(|(k, s)| k == kind && *s == Released)
        };
        // The interface control document and the FMEA are required from the batch tier.
        assert!(!has("CDR", Some("desk"), "icd") && !has("CDR", Some("desk"), "fmea"));
        assert!(has("CDR", Some("batch"), "icd") && has("CDR", Some("batch"), "fmea"));
        // A desk product has no production phase, so its closeout carries none of that phase's documents.
        assert!(!has("CLOSE", Some("desk"), "mfg") && !has("CLOSE", Some("desk"), "usr"));
        assert!(has("CLOSE", Some("batch"), "mfg") && has("CLOSE", Some("batch"), "usr"));
        assert!(!has("CLOSE", Some("batch"), "cmp") && has("CLOSE", Some("retail"), "cmp"));
        // Asked about directly, the production gate still lists its own documents.
        assert!(has("PRR", Some("desk"), "mfg") && !has("PRR", Some("desk"), "cmp"));

        // Without a tier, each kind is listed once, with the tier it applies from.
        let close = product.phase_by_gate("CLOSE").unwrap();
        let every_tier = product.gate_requirements(close, None);
        let usr: Vec<_> = every_tier.iter().filter(|g| g.kind == "usr").collect();
        assert_eq!(usr.len(), 1);
        assert_eq!(usr[0].state, Released);
        assert_eq!(usr[0].min_tier.as_deref(), Some("batch"));
        assert_eq!(usr[0].phase, "P6");
    }

    #[test]
    fn a_gate_runs_the_checks_of_every_phase_up_to_it() {
        let software = Process::get().profile("software").unwrap();
        let checks = |gate: &str| software.gate_checks(software.phase_by_gate(gate).unwrap(), None);
        assert!(checks("CR").is_empty());
        assert_eq!(checks("TRR"), ["requirements-well-formed", "rtm-planned"]);
        assert_eq!(
            checks("RRR"),
            ["requirements-well-formed", "rtm-planned", "rtm-verified"]
        );
        assert_eq!(checks("CLOSE"), checks("RRR"));
    }

    #[test]
    fn baseline_gate_is_first_release() {
        let product = Process::get().profile("product").unwrap();
        assert_eq!(product.baseline_gate("srs", Some("desk")), Some("SRR"));
        assert_eq!(product.baseline_gate("icd", Some("batch")), Some("PDR"));
        assert_eq!(product.baseline_gate("icd", Some("desk")), None);
    }

    #[test]
    fn next_phase_skips_phases_above_the_tier() {
        let product = Process::get().profile("product").unwrap();
        assert_eq!(product.next_phase("P5", Some("desk")).unwrap().id, "P7");
        assert_eq!(product.next_phase("P5", Some("batch")).unwrap().id, "P6");
    }
}
