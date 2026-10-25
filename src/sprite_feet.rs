//! Align the edition's large character art with the unchanged collision bodies.
use bones_bevy_renderer::bevy::{
    asset::HandleId,
    prelude::*,
    render::{Extract, ExtractSchedule, RenderApp},
    sprite::{ExtractedSprites, SpriteSystem},
};
use std::collections::HashMap;

const CELL: usize = 256;
const COLUMNS: usize = 6;
const ROWS: usize = 4;

pub struct FighterFeetPlugin;
impl Plugin for FighterFeetPlugin {
    fn build(&self, app: &mut App) {
        app.sub_app_mut(RenderApp).add_systems(
            ExtractSchedule,
            align_fighter_feet.after(SpriteSystem::ExtractSprites),
        );
    }
}

fn sole_line(data: &[u8], width: usize, frame: usize) -> f32 {
    let left = frame % COLUMNS * CELL;
    let top = frame / COLUMNS * CELL;
    for y in (0..CELL).rev() {
        // Generated alpha can contain a faint halo beyond the actual shoe outline.
        let solid = (left..left + CELL)
            .filter(|x| data[((top + y) * width + x) * 4 + 3] >= 192)
            .take(4)
            .count();
        if solid == 4 {
            return (y + 1) as f32;
        }
    }
    CELL as f32 / 2.0
}

fn foot_anchor(sole: f32, scale: f32, half_height: f32) -> f32 {
    0.5 - sole / CELL as f32 + half_height / (CELL as f32 * scale)
}

fn align_fighter_feet(
    mut sprites: ResMut<ExtractedSprites>,
    images: Extract<Res<Assets<Image>>>,
    mut soles: Local<HashMap<HandleId, [f32; COLUMNS * ROWS]>>,
) {
    for sprite in &mut sprites.sprites {
        let Some(rect) = sprite.rect else { continue };
        if rect.size() != Vec2::splat(CELL as f32) {
            continue;
        }
        let Some(image) = images.get(&Handle::<Image>::weak(sprite.image_handle_id)) else {
            continue;
        };
        let width = image.texture_descriptor.size.width as usize;
        let height = image.texture_descriptor.size.height as usize;
        // This format belongs to the four edition fighters, not terrain, items or guest art.
        if width != CELL * COLUMNS
            || height != CELL * ROWS
            || image.data.len() != width * height * 4
        {
            continue;
        }
        let frame = rect.min.y as usize / CELL * COLUMNS + rect.min.x as usize / CELL;
        if frame >= 17 {
            continue;
        }
        let scale = sprite.transform.to_scale_rotation_translation().0.y.abs();
        if scale <= 0.0 {
            continue;
        }
        let lines = soles
            .entry(sprite.image_handle_id)
            .or_insert_with(|| std::array::from_fn(|i| sole_line(&image.data, width, i)));
        let reference = if frame <= 11 || frame == 14 || frame == 15 {
            frame
        } else {
            0
        };
        let half_height = if frame == 15 { 16.0 } else { 24.0 };
        sprite.anchor.y = foot_anchor(lines[reference], scale, half_height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soles_meet_the_collider_on_every_walk_frame_and_slide() {
        for sole in [219.0, 232.0, 245.0] {
            for half_height in [24.0, 16.0] {
                let anchor = foot_anchor(sole, 0.28, half_height);
                let rendered_sole = (0.5 - sole / CELL as f32 - anchor) * CELL as f32 * 0.28;
                assert!((rendered_sole + half_height).abs() < 0.001);
            }
        }
    }

    #[test]
    fn transparent_padding_and_halos_do_not_lower_the_floor_anchor() {
        let width = CELL * COLUMNS;
        let mut data = vec![0; width * CELL * ROWS * 4];
        for x in 100..120 {
            data[(230 * width + x) * 4 + 3] = 253;
        }
        for x in 100..120 {
            data[(250 * width + x) * 4 + 3] = 16;
        }
        assert_eq!(sole_line(&data, width, 0), 231.0);
    }
}
