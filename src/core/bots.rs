//! Bot skill changes decisions and combat, while platform movement keeps its tested physics.
use crate::prelude::*;

#[derive(HasSchema, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum BotDifficulty {
    #[default]
    Easy,
    Hard,
}

#[derive(HasSchema, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum BotPersonality {
    #[default]
    Mixed,
    Balanced,
    Aggressive,
    Cautious,
    Playful,
}

#[derive(HasSchema, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct BotSettings {
    pub difficulty: BotDifficulty,
    pub personality: BotPersonality,
}

impl BotSettings {
    pub fn resolve(saved: Option<&Self>, defaults: Self) -> Self {
        saved.copied().unwrap_or(defaults)
    }
    pub fn reaction_seconds(self) -> f32 {
        match self.difficulty {
            BotDifficulty::Easy => 0.50,
            BotDifficulty::Hard => 0.10,
        }
    }
    pub fn accuracy(self) -> f64 {
        match self.difficulty {
            BotDifficulty::Easy => 0.55,
            BotDifficulty::Hard => 1.0,
        }
    }
    pub fn decision_seconds(self) -> f32 {
        match self.difficulty {
            BotDifficulty::Easy => 0.65,
            BotDifficulty::Hard => 0.18,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Intent {
    #[default]
    Chase,
    Retreat,
    Explore,
}

#[derive(Clone, Debug, Default)]
pub struct BotBrain {
    applied: Option<BotSettings>,
    pub personality: BotPersonality,
    decision_in: f32,
    idle_for: f32,
    intent: Intent,
    goal: Vec2,
    threat_for: f32,
    attack_in: f32,
    reaction: f32,
}

impl BotBrain {
    pub fn prepare(&mut self, settings: BotSettings, character: BotPersonality, rng: &GlobalRng) {
        if self.applied == Some(settings) {
            return;
        }
        self.personality = match (settings.personality, character) {
            (BotPersonality::Mixed, BotPersonality::Mixed) => [
                BotPersonality::Balanced,
                BotPersonality::Aggressive,
                BotPersonality::Cautious,
                BotPersonality::Playful,
            ][(rng.f32() * 4.0) as usize % 4],
            (BotPersonality::Mixed, style) => style,
            (style, _) => style,
        };
        self.applied = Some(settings);
        self.decision_in = 0.0;
        self.idle_for = 0.0;
        self.threat_for = 0.0;
        self.attack_in = 0.0;
        self.reaction = settings.reaction_seconds() * (0.9 + rng.f32() * 0.4);
    }

    pub fn navigation_goal(
        &mut self,
        settings: BotSettings,
        pos: Vec2,
        target: Vec2,
        velocity: Vec2,
        safe_to_pause: bool,
        dt: f32,
        rng: &GlobalRng,
    ) -> Option<Vec2> {
        self.decision_in -= dt;
        self.idle_for = (self.idle_for - dt).max(0.0);
        if self.idle_for > 0.0 && safe_to_pause {
            return None;
        }
        if self.decision_in <= 0.0 {
            self.decision_in = settings.decision_seconds() * (0.8 + rng.f32() * 0.5);
            let easy = settings.difficulty == BotDifficulty::Easy;
            self.intent = match self.personality {
                BotPersonality::Cautious if pos.distance(target) < 150.0 && rng.chance(0.4) => {
                    Intent::Retreat
                }
                BotPersonality::Playful if rng.chance(if easy { 0.55 } else { 0.30 }) => {
                    Intent::Explore
                }
                _ => Intent::Chase,
            };
            self.goal = match self.intent {
                Intent::Chase => {
                    target
                        + if easy {
                            Vec2::ZERO
                        } else {
                            (velocity * 0.12).clamp_length_max(32.0)
                        }
                }
                Intent::Retreat => pos + vec2(if target.x < pos.x { 112.0 } else { -112.0 }, 0.0),
                Intent::Explore => pos + vec2((rng.f32() - 0.5) * 320.0, (rng.f32() - 0.5) * 128.0),
            };
            let pause_chance = match self.personality {
                BotPersonality::Aggressive => {
                    if easy {
                        0.08
                    } else {
                        0.0
                    }
                }
                BotPersonality::Playful => {
                    if easy {
                        0.35
                    } else {
                        0.15
                    }
                }
                _ => {
                    if easy {
                        0.22
                    } else {
                        0.05
                    }
                }
            };
            if safe_to_pause && rng.chance(pause_chance) {
                self.idle_for = if easy {
                    0.25 + rng.f32() * 0.4
                } else {
                    0.12 + rng.f32() * 0.15
                };
                return None;
            }
        }
        Some(self.goal)
    }

    pub fn wants_to_attack(
        &mut self,
        settings: BotSettings,
        in_range: bool,
        dt: f32,
        rng: &GlobalRng,
    ) -> bool {
        self.attack_in = (self.attack_in - dt).max(0.0);
        if !in_range || self.idle_for > 0.0 {
            self.threat_for = 0.0;
            return false;
        }
        self.threat_for += dt;
        if self.threat_for < self.reaction || self.attack_in > 0.0 {
            return false;
        }
        let aggression = match self.personality {
            BotPersonality::Aggressive => 0.8,
            BotPersonality::Cautious => 1.25,
            _ => 1.0,
        };
        self.attack_in = match settings.difficulty {
            BotDifficulty::Easy => (0.85 + rng.f32() * 0.50) * aggression,
            BotDifficulty::Hard => (0.25 + rng.f32() * 0.25) * aggression,
        };
        rng.chance(settings.accuracy())
    }

    pub fn permits_melee_approach(&self) -> bool {
        self.idle_for <= 0.0 && self.intent == Intent::Chase
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn easy_has_slower_reactions_and_fewer_attack_opportunities() {
        let easy = BotSettings {
            difficulty: BotDifficulty::Easy,
            personality: BotPersonality::Balanced,
        };
        let hard = BotSettings {
            difficulty: BotDifficulty::Hard,
            ..easy
        };
        let simulate = |settings| {
            let rng = GlobalRng::default();
            let mut brain = BotBrain::default();
            brain.prepare(settings, BotPersonality::Mixed, &rng);
            let mut hits = 0;
            let mut first = None;
            for frame in 0..1200 {
                if brain.wants_to_attack(settings, true, 1.0 / FPS, &rng) {
                    hits += 1;
                    first.get_or_insert(frame);
                }
            }
            (hits, first.unwrap())
        };
        let (easy_hits, easy_first) = simulate(easy);
        let (hard_hits, hard_first) = simulate(hard);
        assert!(easy_first > hard_first + 15);
        assert!(easy_hits * 2 < hard_hits);
        assert_eq!(BotSettings::default().difficulty, BotDifficulty::Easy);
    }
    #[test]
    fn personalities_make_distinct_choices_and_never_pause_a_committed_route() {
        let observe = |style| {
            let rng = GlobalRng::default();
            let settings = BotSettings {
                personality: style,
                ..default()
            };
            let mut brain = BotBrain::default();
            brain.prepare(settings, BotPersonality::Mixed, &rng);
            brain.idle_for = 0.4;
            assert!(brain
                .navigation_goal(
                    settings,
                    Vec2::ZERO,
                    vec2(80.0, 0.0),
                    Vec2::ZERO,
                    false,
                    1.0 / FPS,
                    &rng
                )
                .is_some());
            let mut retreats = 0;
            let mut explorations = 0;
            for _ in 0..180 {
                assert!(brain
                    .navigation_goal(
                        settings,
                        Vec2::ZERO,
                        vec2(80.0, 0.0),
                        Vec2::ZERO,
                        false,
                        1.0,
                        &rng
                    )
                    .is_some());
                retreats += (brain.intent == Intent::Retreat) as u32;
                explorations += (brain.intent == Intent::Explore) as u32;
            }
            (retreats, explorations)
        };
        assert_eq!(observe(BotPersonality::Aggressive), (0, 0));
        assert!(observe(BotPersonality::Cautious).0 > 25);
        assert!(observe(BotPersonality::Playful).1 > 25);
    }
    #[test]
    fn remix_defaults_character_styles_and_saved_overrides_work_together() {
        let defaults = BotSettings {
            difficulty: BotDifficulty::Hard,
            personality: BotPersonality::Mixed,
        };
        assert_eq!(BotSettings::resolve(None, defaults), defaults);
        let saved = BotSettings::default();
        assert_eq!(BotSettings::resolve(Some(&saved), defaults), saved);
        let mut brain = BotBrain::default();
        let rng = GlobalRng::default();
        brain.prepare(defaults, BotPersonality::Cautious, &rng);
        assert_eq!(brain.personality, BotPersonality::Cautious);
        brain.prepare(
            BotSettings {
                personality: BotPersonality::Playful,
                ..defaults
            },
            BotPersonality::Cautious,
            &rng,
        );
        assert_eq!(brain.personality, BotPersonality::Playful);
    }
    #[test]
    fn missing_new_fields_load_with_easy_mixed_defaults() {
        use serde::de::DeserializeSeed;
        let parsed = SchemaDeserializer(BotSettings::schema())
            .deserialize(serde_yaml::Deserializer::from_str("{}"))
            .unwrap();
        assert_eq!(*parsed.cast_ref::<BotSettings>(), BotSettings::default());
        let configured = SchemaDeserializer(BotSettings::schema())
            .deserialize(serde_yaml::Deserializer::from_str(
                "difficulty: Hard\npersonality: Playful\n",
            ))
            .unwrap();
        assert_eq!(
            *configured.cast_ref::<BotSettings>(),
            BotSettings {
                difficulty: BotDifficulty::Hard,
                personality: BotPersonality::Playful
            }
        );
        let fighter = SchemaDeserializer(PlayerMeta::schema())
            .deserialize(serde_yaml::Deserializer::from_str(
                "name: Rivet\nbot_personality: Aggressive\n",
            ))
            .unwrap();
        assert_eq!(
            fighter.cast_ref::<PlayerMeta>().bot_personality,
            BotPersonality::Aggressive
        );
    }
}
