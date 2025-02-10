use std::f32::consts::FRAC_PI_2;

use nalgebra::{Matrix4, Unit, UnitQuaternion, Vector3};
use vulkano::{buffer::BufferContents, pipeline::graphics::vertex_input::Vertex};

const NUM_INSTANCES_PER_ROW: u32 = 4;
const INSTANCE_DISPLACEMENT: Vector3<f32> = Vector3::new(
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
    0.0,
);
const SPACE_BETWEEN: f32 = 2.0;
pub struct Instance {
    position: Vector3<f32>,
    rotation: UnitQuaternion<f32>,
}

impl Instance {
    pub fn new() -> Vec<Instance> {
        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|y| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let y = SPACE_BETWEEN * (y as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                    let position: Vector3<f32> =
                        Vector3::new(x as f32, y as f32, 0.0) - INSTANCE_DISPLACEMENT;

                    let rotation = if position == Vector3::zeros() {
                        // this is needed so an object at (0, 0, 0) won't get scaled to zero
                        // as Quaternions can affect scale if they're not created correctly
                        UnitQuaternion::from_axis_angle(&Vector3::<f32>::z_axis(), 0.0)
                    } else {
                        UnitQuaternion::from_axis_angle(&Unit::new_normalize(position), FRAC_PI_2)
                    };

                    Instance { position, rotation }
                })
            })
            .collect::<Vec<_>>();

        instances
    }
}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        let full_matrix: [[f32; 4]; 4] =
            (Matrix4::new_translation(&self.position) * self.rotation.to_homogeneous()).into();
        InstanceRaw {
            matrix1: full_matrix[0],
            matrix2: full_matrix[1],
            matrix3: full_matrix[2],
            matrix4: full_matrix[3],
        }
    }
}

// Split matrix to be able to match the Vertex format
#[repr(C)]
#[derive(Copy, Clone, BufferContents, Vertex)]
pub struct InstanceRaw {
    #[format(R32G32B32A32_SFLOAT)]
    pub matrix1: [f32; 4],
    #[format(R32G32B32A32_SFLOAT)]
    pub matrix2: [f32; 4],
    #[format(R32G32B32A32_SFLOAT)]
    pub matrix3: [f32; 4],
    #[format(R32G32B32A32_SFLOAT)]
    pub matrix4: [f32; 4],
}
