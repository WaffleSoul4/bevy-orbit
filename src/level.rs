use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct LevelDescriptor {
    pub entities: Vec<EntityType>,
    pub starting_position: Vec2,
    pub name: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum EntityType {
    StaticObject {
        position: Vec2,
        mass: f32,
        gravitator: bool,
        color: Color,
    },
    Trigger {
        position: Vec2,
    },
}

impl EntityType {
    pub fn new_static(position: Vec2, mass: f32, gravitator: bool, color: Color) -> Self {
        EntityType::StaticObject {
            position,
            mass,
            gravitator,
            color,
        }
    }
}

impl LevelDescriptor {
    pub fn load_from_file(path: PathBuf) -> Result<Self, Box<dyn Error>> {
        let file = File::open(path)?;
        let buf_reader = BufReader::new(file);

        let level_descriptor = serde_json::from_reader(buf_reader)?;

        Ok(level_descriptor)
    }

    pub fn save_to_file(&self, path: PathBuf) -> Result<(), Box<dyn Error>> {
        let file = File::create(path)?;
        let mut buf_writer = BufWriter::new(file);

        serde_json::to_writer(&mut buf_writer, self)?;

        buf_writer.flush()?;

        Ok(())
    }

    pub fn add_entity(&mut self, entity: EntityType) -> &mut Self {
        self.entities.push(entity);
        self
    }

    pub fn new(starting_position: Vec2, name: &str) -> Self {
        LevelDescriptor {
            entities: vec![],
            starting_position,
            name: name.to_string(),
        }
    }
}

impl Default for LevelDescriptor {
    fn default() -> Self {
        LevelDescriptor {
            entities: vec![],
            starting_position: Vec2::ZERO,
            name: "Default Name".to_string(),
        }
    }
}
