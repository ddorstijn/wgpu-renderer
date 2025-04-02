pub fn load_glb(file_path: &str) -> (Vec<[f32; 3]>, Vec<u32>) {
    // Open the .glb file
    let (gltf, buffers, _) = gltf::import(file_path).expect("Failed to load GLB file");

    gltf.meshes()
        .flat_map(|mesh| {
            mesh.primitives().flat_map(|primitive| {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                let positions = reader
                    .read_positions()
                    .map(|p| p.map(|v| [v[0], v[1], v[2]]))
                    .into_iter()
                    .flatten();
                let indices = reader
                    .read_indices()
                    .map(|i| i.into_u32())
                    .into_iter()
                    .flatten();
                positions.zip(indices).map(|(v, i)| (v, i))
            })
        })
        .unzip()
}

pub fn load_obj(file_path: &str) -> (Vec<[f32; 3]>, Vec<u32>) {
    // Load the OBJ file
    let (models, _) =
        tobj::load_obj(file_path, &tobj::LoadOptions::default()).expect("Failed to load OBJ file");

    let mut positions = Vec::new();
    let mut indices = Vec::new();

    for model in models {
        let mesh = model.mesh;

        // Collect vertex positions
        for i in 0..mesh.positions.len() / 3 {
            positions.push([
                mesh.positions[i * 3],
                mesh.positions[i * 3 + 1],
                mesh.positions[i * 3 + 2],
            ]);
        }

        // Collect indices
        indices.extend(&mesh.indices);
    }

    (positions, indices)
}
