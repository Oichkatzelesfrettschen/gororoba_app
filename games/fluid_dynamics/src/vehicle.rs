// Vehicle hull designer: voxel editing and preset shapes.
//
// Provides preset vehicle shapes and a voxel editor component
// for designing custom hulls in the VehicleDesign state.

use bevy::prelude::*;
use gororoba_bevy_lbm::VoxelGrid;

/// Preset vehicle shapes that can be placed in the wind tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehiclePreset {
    /// Sphere: high drag baseline.
    Sphere,
    /// Wedge: moderate streamlining.
    Wedge,
    /// Airfoil: low drag, high lift.
    Airfoil,
}

/// Marker component for the active vehicle entity.
#[derive(Component)]
pub struct Vehicle {
    /// Which preset generated this vehicle (used for UI display).
    #[allow(dead_code)]
    pub preset: Option<VehiclePreset>,
}

/// Generate a VoxelGrid with a sphere centered in the domain.
pub fn sphere_voxels(nx: usize, ny: usize, nz: usize, radius: f32) -> VoxelGrid {
    let mut grid = VoxelGrid::new(nx, ny, nz);
    let cx = nx as f32 / 2.0;
    let cy = ny as f32 / 2.0;
    let cz = nz as f32 / 2.0;
    let r2 = radius * radius;

    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                let dz = z as f32 + 0.5 - cz;
                if dx * dx + dy * dy + dz * dz <= r2 {
                    grid.set(x, y, z, true);
                }
            }
        }
    }
    grid
}

/// Generate a VoxelGrid with a wedge shape (triangular cross-section).
///
/// The wedge points in the -X direction (into the flow) for streamlining.
pub fn wedge_voxels(nx: usize, ny: usize, nz: usize) -> VoxelGrid {
    let mut grid = VoxelGrid::new(nx, ny, nz);
    let cx = nx / 2;
    let cy = ny / 2;
    let cz = nz / 2;
    let half_width = ny / 6;
    let half_depth = nz / 6;
    let length = nx / 3;

    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let ix = x as i32 - cx as i32;
                let iy = (y as i32 - cy as i32).unsigned_abs() as usize;
                let iz = (z as i32 - cz as i32).unsigned_abs() as usize;

                if ix >= -(length as i32) && ix <= 0 {
                    // Taper: at ix=0, full width; at ix=-length, zero width.
                    let progress = (ix + length as i32) as f32 / length as f32;
                    let max_y = (half_width as f32 * progress) as usize;
                    let max_z = (half_depth as f32 * progress) as usize;
                    if iy <= max_y && iz <= max_z {
                        grid.set(x, y, z, true);
                    }
                }
            }
        }
    }
    grid
}

/// Generate a VoxelGrid with a simple airfoil cross-section extruded in Z.
///
/// Uses an elliptical cross-section with tapered trailing edge.
pub fn airfoil_voxels(nx: usize, ny: usize, nz: usize) -> VoxelGrid {
    let mut grid = VoxelGrid::new(nx, ny, nz);
    let cx = nx / 2;
    let cy = ny / 2;
    let cz = nz / 2;
    let chord = nx / 3; // chord length
    let thickness = ny / 8; // max half-thickness
    let span = nz / 3; // half-span in Z

    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let iz = (z as i32 - cz as i32).unsigned_abs() as usize;
                if iz > span {
                    continue;
                }

                let ix = x as i32 - (cx as i32 - chord as i32 / 2);
                if ix < 0 || ix >= chord as i32 {
                    continue;
                }

                // NACA-like thickness distribution: t(x) = t_max * sqrt(1 - (2x/c - 1)^2)
                let frac = ix as f32 / chord as f32;
                let t = thickness as f32 * (1.0 - (2.0 * frac - 1.0).powi(2)).sqrt();
                let iy = (y as i32 - cy as i32).unsigned_abs() as f32;

                if iy <= t {
                    grid.set(x, y, z, true);
                }
            }
        }
    }
    grid
}

/// Generate a VoxelGrid from a preset.
pub fn preset_voxels(preset: VehiclePreset, nx: usize, ny: usize, nz: usize) -> VoxelGrid {
    match preset {
        VehiclePreset::Sphere => sphere_voxels(nx, ny, nz, (nx.min(ny).min(nz) / 6) as f32),
        VehiclePreset::Wedge => wedge_voxels(nx, ny, nz),
        VehiclePreset::Airfoil => airfoil_voxels(nx, ny, nz),
    }
}

/// Spawn voxel mesh visualization as gizmo cubes.
///
/// Called each frame to draw the vehicle hull as colored wireframe cubes.
pub fn vehicle_gizmo_system(query: Query<&VoxelGrid, With<Vehicle>>, mut gizmos: Gizmos) {
    for grid in &query {
        for z in 0..grid.nz {
            for y in 0..grid.ny {
                for x in 0..grid.nx {
                    if grid.get(x, y, z) {
                        let pos = Vec3::new(
                            x as f32 - grid.nx as f32 / 2.0 + 0.5,
                            y as f32 - grid.ny as f32 / 2.0 + 0.5,
                            z as f32 - grid.nz as f32 / 2.0 + 0.5,
                        );
                        gizmos.cube(
                            Transform::from_translation(pos).with_scale(Vec3::splat(0.9)),
                            Color::srgba(0.2, 0.6, 0.9, 0.7),
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_has_solid_cells() {
        let grid = sphere_voxels(16, 16, 16, 4.0);
        assert!(grid.solid_count() > 0);
        // Center should be solid.
        assert!(grid.get(8, 8, 8));
        // Corner should be fluid.
        assert!(!grid.get(0, 0, 0));
    }

    #[test]
    fn wedge_has_solid_cells() {
        let grid = wedge_voxels(32, 32, 32);
        assert!(grid.solid_count() > 0);
    }

    #[test]
    fn airfoil_has_solid_cells() {
        let grid = airfoil_voxels(32, 32, 32);
        assert!(grid.solid_count() > 0);
    }

    #[test]
    fn preset_generates_all() {
        for preset in [
            VehiclePreset::Sphere,
            VehiclePreset::Wedge,
            VehiclePreset::Airfoil,
        ] {
            let grid = preset_voxels(preset, 32, 32, 32);
            assert!(
                grid.solid_count() > 0,
                "preset {preset:?} should produce solid cells"
            );
        }
    }

    #[test]
    fn sphere_symmetry() {
        let n = 16;
        let grid = sphere_voxels(n, n, n, 4.0);
        // Check approximate symmetry: count solid in each octant should be similar.
        let half = n / 2;
        let mut count_low = 0;
        let mut count_high = 0;
        for z in 0..n {
            for y in 0..n {
                for x in 0..n {
                    if grid.get(x, y, z) {
                        if x < half {
                            count_low += 1;
                        } else {
                            count_high += 1;
                        }
                    }
                }
            }
        }
        // Should be roughly equal (within 10% of total).
        let total = grid.solid_count();
        let diff = (count_low as f64 - count_high as f64).abs();
        assert!(
            diff < total as f64 * 0.1,
            "sphere should be symmetric: low={count_low} high={count_high}"
        );
    }
}
