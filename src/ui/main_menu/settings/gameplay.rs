use super::*;

pub(super) fn widget(
    In((ui, state, reset)): In<(&mut egui::Ui, &mut SettingsState, bool)>,
    meta: Root<GameMeta>,
    localization: Localization<GameMeta>,
) {
    if reset {
        state.modified_bots = meta.bot_settings;
        state.reset_bot_override = true;
    }
    let font = meta
        .theme
        .font_styles
        .normal
        .with_color(meta.theme.panel.font_color);
    ui.add_space(font.size);
    egui::Grid::new("bot-settings")
        .spacing([font.size, font.size / 2.0])
        .show(ui, |ui| {
            ui.label(font.rich(localization.get("bot-difficulty")));
            let selected = match state.modified_bots.difficulty {
                BotDifficulty::Easy => "bot-easy",
                BotDifficulty::Hard => "bot-hard",
            };
            egui::ComboBox::from_id_source("bot-difficulty")
                .selected_text(localization.get(selected))
                .show_ui(ui, |ui| {
                    for (value, key) in [
                        (BotDifficulty::Easy, "bot-easy"),
                        (BotDifficulty::Hard, "bot-hard"),
                    ] {
                        if ui
                            .selectable_value(
                                &mut state.modified_bots.difficulty,
                                value,
                                localization.get(key),
                            )
                            .changed()
                        {
                            state.reset_bot_override = false;
                        }
                    }
                });
            ui.end_row();
            ui.label(font.rich(localization.get("bot-personality")));
            let styles = [
                (BotPersonality::Mixed, "bot-mixed"),
                (BotPersonality::Balanced, "bot-balanced"),
                (BotPersonality::Aggressive, "bot-aggressive"),
                (BotPersonality::Cautious, "bot-cautious"),
                (BotPersonality::Playful, "bot-playful"),
            ];
            let selected = styles
                .iter()
                .find(|(value, _)| *value == state.modified_bots.personality)
                .unwrap()
                .1;
            egui::ComboBox::from_id_source("bot-personality")
                .selected_text(localization.get(selected))
                .show_ui(ui, |ui| {
                    for (value, key) in styles {
                        if ui
                            .selectable_value(
                                &mut state.modified_bots.personality,
                                value,
                                localization.get(key),
                            )
                            .changed()
                        {
                            state.reset_bot_override = false;
                        }
                    }
                });
            ui.end_row();
        });
    ui.add_space(font.size);
    let difficulty_help = match state.modified_bots.difficulty {
        BotDifficulty::Easy => "bot-easy-help",
        BotDifficulty::Hard => "bot-hard-help",
    };
    ui.label(font.rich(localization.get(difficulty_help)));
    let personality_help = match state.modified_bots.personality {
        BotPersonality::Mixed => "bot-mixed-help",
        BotPersonality::Balanced => "bot-balanced-help",
        BotPersonality::Aggressive => "bot-aggressive-help",
        BotPersonality::Cautious => "bot-cautious-help",
        BotPersonality::Playful => "bot-playful-help",
    };
    ui.label(font.rich(localization.get(personality_help)));
    ui.add_space(font.size);
    ui.label(font.rich(localization.get("bot-local-help")));
}
