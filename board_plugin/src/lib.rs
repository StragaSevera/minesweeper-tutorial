pub mod components;
pub mod resources;

use crate::resources::tile::Tile;
use crate::{
    components::*,
    resources::{tile_map::TileMap, BoardOptions, BoardPosition, TileSize},
};
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
#[cfg(feature = "debug")]
use bevy_inspector_egui::RegisterInspectable;

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(Self::create_board);
        info!("Loaded Board Plugin");

        #[cfg(feature = "debug")]
        {
            // registering custom component to be able to edit it in inspector
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
                );
            });
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
    ) {
        // Tiles
        for (y, line) in tile_map.iter().enumerate() {
            for (x, tile) in line.iter().enumerate() {
                let coordinates = Coordinates { x: x as u16, y: y as u16 };
                let mut cmd = parent.spawn();
                Self::insert_tile(&mut cmd, padding, size, y, x, coordinates, color);

                match tile {
                    Tile::Bomb => {
                        Self::insert_bomb(&mut cmd, &bomb_image, padding, size);
                    }
                    Tile::BombNeighbor(count) => {
                        Self::insert_bomb_neighbor(&mut cmd, &font, *count, size, padding);
                    }
                    Tile::Empty => (),
                }
            }
        }
    }

    //noinspection RsTypeCheck
    fn insert_bomb(cmd: &mut EntityCommands, bomb_image: &Handle<Image>, padding: f32, size: f32) {
        // If the tile is a bomb we add the matching component and a sprite child
        cmd.insert(Bomb);
        cmd.with_children(|parent| {
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
        cmd: &mut EntityCommands,
        font: &Handle<Font>,
        count: u8,
        size: f32,
        padding: f32,
    ) {
        // If the tile is a bomb neighbour we add the matching component and a text child
        cmd.insert(BombNeighbor { count });
        cmd.with_children(|parent| {
            parent.spawn_bundle(Self::bomb_count_text_bundle(count, font.clone(), size - padding));
        });
    }

    fn insert_tile(
        cmd: &mut EntityCommands,
        padding: f32,
        size: f32,
        y: usize,
        x: usize,
        coordinates: Coordinates,
        color: Color,
    ) {
        cmd.insert_bundle(SpriteBundle {
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
