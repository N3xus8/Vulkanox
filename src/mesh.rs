use std::iter::zip;

use gltf::Gltf;
use tracing::warn;

use crate::error::Result;
use crate::shader::Vertex;
pub struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    indices: Vec<u16>,
    normals: Option<Vec<[f32; 3]>>,
    uvs: Option<Vec<[f32; 2]>>,
}

impl MeshBuilder {
    pub fn read_gltf(path: &str) -> Result<MeshBuilder> {
        //"assets/Box.gltf"
        let gltf = Gltf::open(path)?;
        for scene in gltf.scenes() {
            for node in scene.nodes() {
                println!(
                    "Node #{} has {} children",
                    node.index(),
                    node.children().count(),
                );
            }
        }

        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut indices = Vec::new();
        let mut uvs = Vec::new();
        let mut normals = Vec::new();
        let mut joint_indices = Vec::new();
        let mut joint_weigths = Vec::new();

        let (gltf, buffers, _) = gltf::import("assets/Box.gltf")?;
        for mesh in gltf.meshes() {
            println!("Mesh #{}", mesh.index());
            for primitive in mesh.primitives() {
                println!("- Primitive #{}", primitive.index());
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                // Positions
                if let Some(iter) = reader.read_positions() {
                    for vertex_position in iter {
                        //   println!("{:?}", vertex_position);
                        positions.push(vertex_position);
                    }
                }
                // Indices

                if let Some(gltf::mesh::util::ReadIndices::U16(gltf::accessor::Iter::Standard(
                    iter,
                ))) = reader.read_indices()
                {
                    for indice in iter {
                        //    println!("{:?}", indice);
                        indices.push(indice);
                    }
                }

                if let Some(gltf::mesh::util::ReadTexCoords::F32(gltf::accessor::Iter::Standard(
                    iter,
                ))) = reader.read_tex_coords(0)
                {
                    for texture_coord in iter {
                        uvs.push(texture_coord);
                    }
                }
                if let Some(iter) = reader.read_normals() {
                    for normal in iter {
                        normals.push(normal);
                    }
                }
                if let Some(gltf::mesh::util::ReadJoints::U8(gltf::accessor::Iter::Standard(
                    iter,
                ))) = reader.read_joints(0)
                {
                    for joint_indice in iter {
                        joint_indices.push(joint_indice);
                    }
                }
                if let Some(gltf::mesh::util::ReadWeights::F32(gltf::accessor::Iter::Standard(
                    iter,
                ))) = reader.read_weights(0)
                {
                    for joint_weigth in iter {
                        joint_weigths.push(joint_weigth);
                    }
                }
            }
        }

        //let indices = if indices.len() == 0 { None } else {Some(indices)};
        let normals = if normals.is_empty() {
            None
        } else {
            Some(normals)
        };
        let uvs = if uvs.is_empty() { None } else { Some(uvs) };

        Ok(MeshBuilder {
            positions,
            normals,
            indices,
            uvs,
        })
    }

    pub fn vertices(&self) -> Result<Vec<Vertex>> {
        let mut vertices = Vec::<Vertex>::new();

        match &self.normals {
            Some(normals) => {
                for (position, normal) in self.positions.iter().zip(normals.iter()) {
                    vertices.push(Vertex {
                        position: *position,
                        normal: *normal,
                    });
                }
            }
            None => {
                for position in &self.positions {
                    warn!("no normal found. compute default");
                    vertices.push(Vertex {
                        position: *position,
                        normal: [0., 0., 1.],
                    });
                }
            }
        }

        Ok(vertices)
    }

    pub fn indices(&self) -> Vec<u16> {
        self.indices.clone()
    }
}
