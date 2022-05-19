mod bounds;
mod components;
mod events;
pub mod resources;
mod systems;

use crate::{
    bounds::Bounds2,
    components::*,
    events::TileTriggerEvent,
    resources::{tile::Tile, tile_map::TileMap, Board, BoardOptions, BoardPosition, TileSize},
    systems::{
        input::input_handling,
        uncover::{trigger_event_handler, uncover_tiles},
    },
};
use bevy::utils::HashMap;
use bevy::{ecs::system::EntityCommands, math::Vec3Swizzles, prelude::*};
#[cfg(feature = "debug")]
use bevy_inspector_egui::RegisterInspectable;

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(Self::create_board)
            .add_system(input_handling)
            .add_event::<TileTriggerEvent>()
            .add_system(trigger_event_handler)
            .add_system(uncover_tiles);
        info!("Loaded Board Plugin");

        // registering custom components to be able to edit it in inspector
        #[cfg(feature = "debug")]
        {
            app.register_inspectable::<Coordinates>();
            app.register_inspectable::<BombNeighbor>();
            app.register_inspectable::<Bomb>();
            app.register_inspectable::<Uncover>();
        }
    }
}

impl BoardPlugin {
    /// System to generate the complete board
    pub fn create_board(
        mut commands: Commands,
        board_options: Option<Res<BoardOptions>>,
        window: Res<WindowDescriptor>,
        asset_server: Res<AssetServer>,
    ) {
        let font = asset_server.load("fonts/pixeled.ttf");
        let bomb_image = asset_server.load("sprites/bomb.png");
        let options = match board_options {
            None => BoardOptions::default(), // If no options is set we use the default one
            Some(o) => o.clone(),
        };

        let tile_map = Self::build_map(&options);
        let tile_size = Self::build_tile_size(window, &options, &tile_map);
        let board_size =
            Vec2::new(tile_map.width() as f32 * tile_size, tile_map.height() as f32 * tile_size);
        let board_position = Self::build_board_position(&options, board_size);
        let mut covered_tiles =
            HashMap::with_capacity((tile_map.width() * tile_map.height()).into());
        let mut safe_start = None;

        commands
            .spawn()
            .insert(Name::new("Board"))
            .insert(Transform::from_translation(board_position))
            .insert(GlobalTransform::default())
            .with_children(|parent| {
                Self::spawn_background(board_size, parent);
                Self::spawn_tiles(
                    parent,
                    &tile_map,
                    tile_size,
                    options.tile_padding,
                    Color::GRAY,
                    bomb_image,
                    font,
                    Color::DARK_GRAY,
                    &mut covered_tiles,
                    &mut safe_start,
                );
            });
        commands.insert_resource(Board {
            tile_map,
            tile_size,
            covered_tiles,
            bounds: Bounds2 { position: board_position.xy(), size: board_size },
        });
        if options.safe_start {
            if let Some(entity) = safe_start {
                commands.entity(entity).insert(Uncover);
            }
        }
    }

    fn build_map(options: &BoardOptions) -> TileMap {
        let mut tile_map = TileMap::empty(options.map_size.0, options.map_size.1);
        tile_map.set_bombs(options.bomb_count);
        #[cfg(feature = "debug")]
        info!("{}", tile_map.console_output());
        tile_map
    }

    fn build_tile_size(
        window: Res<WindowDescriptor>,
        options: &BoardOptions,
        tile_map: &TileMap,
    ) -> f32 {
        match options.tile_size {
            TileSize::Fixed(v) => v,
            TileSize::Adaptive { min, max } => Self::adaptative_tile_size(
                window,
                (min, max),
                (tile_map.width(), tile_map.height()),
            ),
        }
    }

    /// Board anchor position (bottom left)
    fn build_board_position(options: &BoardOptions, board_size: Vec2) -> Vec3 {
        match options.position {
            BoardPosition::Centered { offset } => {
                Vec3::new(-(board_size.x / 2.), -(board_size.y / 2.), 0.) + offset
            }
            BoardPosition::Custom(p) => p,
        }
    }

    fn spawn_background(board_size: Vec2, parent: &mut ChildBuilder) {
        // We spawn the board background sprite at the center of the board,
        // since the sprite pivot is centered
        parent
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    color: Color::WHITE,
                    custom_size: Some(board_size),
                    ..Default::default()
                },
                transform: Transform::from_xyz(board_size.x / 2., board_size.y / 2., 0.),
                ..Default::default()
            })
            .insert(Name::new("Background"));
    }

    // TODO: Refactor this to builder
    fn spawn_tiles(
        parent: &mut ChildBuilder,
        tile_map: &TileMap,
        size: f32,
        padding: f32,
        color: Color,
        bomb_image: Handle<Image>,
        font: Handle<Font>,
        covered_tile_color: Color,
        covered_tiles: &mut HashMap<Coordinates, Entity>,
        safe_start_entity: &mut Option<Entity>,
    ) {
        // Tiles
        for (y, line) in tile_map.iter().enumerate() {
            for (x, tile) in line.iter().enumerate() {
                let coordinates = Coordinates { x: x as u16, y: y as u16 };
                let mut tile_entity = parent.spawn(); // Ex: cmd
                                                      // Refactor to struct VisualTile
                Self::insert_tile(
                    &mut tile_entity,
                    padding,
                    size,
                    y,
                    x,
                    coordinates,
                    color,
                    covered_tile_color,
                    covered_tiles,
                    safe_start_entity,
                    tile,
                );

                match tile {
                    Tile::Bomb => {
                        Self::insert_bomb(&mut tile_entity, &bomb_image, padding, size);
                    }
                    Tile::BombNeighbor(count) => {
                        Self::insert_bomb_neighbor(&mut tile_entity, &font, *count, size, padding);
                    }
                    Tile::Empty => (),
                }
            }
        }
    }

    //noinspection RsTypeCheck
    fn insert_bomb(
        tile_entity: &mut EntityCommands,
        bomb_image: &Handle<Image>,
        padding: f32,
        size: f32,
    ) {
        // If the tile is a bomb we add the matching component and a sprite child
        tile_entity.insert(Bomb);
        tile_entity.with_children(|parent| {
            parent.spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(size - padding)),
                    ..Default::default()
                },
                transform: Transform::from_xyz(0., 0., 1.),
                texture: bomb_image.clone(),
                ..Default::default()
            });
        });
    }

    //noinspection RsTypeCheck
    fn insert_bomb_neighbor(
        tile_entity: &mut EntityCommands,
        font: &Handle<Font>,
        count: u8,
        size: f32,
        padding: f32,
    ) {
        // If the tile is a bomb neighbour we add the matching component and a text child
        tile_entity.insert(BombNeighbor { count });
        tile_entity.with_children(|parent| {
            parent.spawn_bundle(Self::bomb_count_text_bundle(count, font.clone(), size - padding));
        });
    }

    fn insert_tile(
        tile_entity: &mut EntityCommands,
        padding: f32,
        size: f32,
        y: usize,
        x: usize,
        coordinates: Coordinates,
        color: Color,
        covered_tile_color: Color,
        covered_tiles: &mut HashMap<Coordinates, Entity>,
        safe_start_entity: &mut Option<Entity>,
        tile: &Tile,
    ) {
        tile_entity
            .insert_bundle(SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::splat(size - padding)),
                    ..Default::default()
                },
                transform: Transform::from_xyz(
                    (x as f32 * size) + (size / 2.),
                    (y as f32 * size) + (size / 2.),
                    1.,
                ),
                ..Default::default()
            })
            .insert(Name::new(format!("Tile ({}, {})", x, y)))
            .insert(coordinates);
        Self::insert_cover(
            tile_entity,
            covered_tiles,
            covered_tile_color,
            padding,
            size,
            coordinates,
            safe_start_entity,
            tile,
        );
    }

    fn insert_cover(
        tile_entity: &mut EntityCommands,
        covered_tiles: &mut HashMap<Coordinates, Entity>,
        covered_tile_color: Color,
        padding: f32,
        size: f32,
        coordinates: Coordinates,
        safe_start_entity: &mut Option<Entity>,
        tile: &Tile,
    ) {
        tile_entity.with_children(|parent| {
            let entity = parent
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(size - padding)),
                        color: covered_tile_color,
                        ..Default::default()
                    },
                    transform: Transform::from_xyz(0., 0., 2.),
                    ..Default::default()
                })
                .insert(Name::new("Tile Cover"))
                .id();
            covered_tiles.insert(coordinates, entity);
            if safe_start_entity.is_none() && *tile == Tile::Empty {
                *safe_start_entity = Some(entity);
            }
        });
    }

    /// Generates the bomb counter text 2D Bundle for a given value
    fn bomb_count_text_bundle(count: u8, font: Handle<Font>, size: f32) -> Text2dBundle {
        // We retrieve the text and the correct color
        let (text, color) = (
            count.to_string(),
            match count {
                1 => Color::WHITE,
                2 => Color::GREEN,
                3 => Color::YELLOW,
                4 => Color::ORANGE,
                _ => Color::PURPLE,
            },
        );
        // We generate a text bundle
        Text2dBundle {
            text: Text {
                sections: vec![TextSection {
                    value: text,
                    style: TextStyle { color, font, font_size: size },
                }],
                alignment: TextAlignment {
                    vertical: VerticalAlign::Center,
                    horizontal: HorizontalAlign::Center,
                },
            },
            transform: Transform::from_xyz(0., 0., 1.),
            ..Default::default()
        }
    }

    /// Computes a tile size that matches the window according to the tile map size
    fn adaptative_tile_size(
        window: Res<WindowDescriptor>,
        (min, max): (f32, f32),      // Tile size constraints
        (width, height): (u16, u16), // Tile map dimensions
    ) -> f32 {
        let max_width = window.width / width as f32;
        let max_heigth = window.height / height as f32;
        max_width.min(max_heigth).clamp(min, max)
    }
}
