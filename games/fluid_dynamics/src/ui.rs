// Game-specific UI: menu, simulation controls, parameter tweaking,
// velocity field visualization, and pedagogy content.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};

use gororoba_bevy_core::{EguiReady, PedagogyMode, PedagogyState};
use gororoba_bevy_lbm::{LbmCpuEngine, SimulationDiagnostics, VoxelGrid};

use crate::aerodynamics::AerodynamicResults;
use crate::scenarios::{WindTunnelConfig, WindTunnelDomain};
use crate::states::{FluidGamePhase, FluidSimState};
use crate::vehicle::{Vehicle, VehiclePreset, preset_voxels};

/// Plugin for game-specific UI systems.
pub struct FluidUiPlugin;

impl Plugin for FluidUiPlugin {
    fn build(&self, app: &mut App) {
        let egui_ready = resource_exists::<EguiReady>;
        app.add_systems(Startup, setup_pedagogy)
            .add_systems(
                EguiPrimaryContextPass,
                menu_ui_system
                    .run_if(in_state(FluidGamePhase::Menu))
                    .run_if(egui_ready),
            )
            .add_systems(
                EguiPrimaryContextPass,
                design_ui_system
                    .run_if(in_state(FluidSimState::VehicleDesign))
                    .run_if(egui_ready),
            )
            .add_systems(
                EguiPrimaryContextPass,
                tunnel_ui_system
                    .run_if(in_state(FluidSimState::WindTunnel))
                    .run_if(egui_ready),
            )
            .add_systems(
                EguiPrimaryContextPass,
                results_ui_system
                    .run_if(in_state(FluidSimState::Results))
                    .run_if(egui_ready),
            )
            .add_systems(
                Update,
                velocity_gizmo_system.run_if(in_state(FluidSimState::WindTunnel)),
            );
    }
}

/// Register pedagogy content about LBM and fluid dynamics.
fn setup_pedagogy(mut pedagogy: ResMut<PedagogyState>) {
    pedagogy.add(
        PedagogyMode::Story,
        "The Wind Tunnel",
        "Design a vehicle and test it in a virtual wind tunnel. \
         Watch how air flows around your creation and learn about \
         drag, lift, and turbulence.",
    );
    pedagogy.add(
        PedagogyMode::Explorer,
        "Lattice Boltzmann Method",
        "Instead of solving the Navier-Stokes equations directly, \
         LBM simulates fluid as particles streaming and colliding \
         on a discrete lattice. Each cell tracks 19 velocity \
         distributions (D3Q19 lattice).",
    );
    pedagogy.add(
        PedagogyMode::Explorer,
        "Reynolds Number",
        "Re = u * L / nu. Low Re (< 1000): laminar flow. \
         High Re (> 10000): turbulent flow. The relaxation \
         time tau controls viscosity: nu = (tau - 0.5) / 3.",
    );
    pedagogy.add(
        PedagogyMode::Research,
        "BGK Collision Operator",
        "f_i(x + c_i*dt, t + dt) = f_i(x, t) - (f_i - f_i^eq) / tau\n\n\
         The Bhatnagar-Gross-Krook operator relaxes distributions \
         toward local equilibrium at rate 1/tau. Stability requires \
         tau > 0.5.",
    );
    pedagogy.add(
        PedagogyMode::Research,
        "Drag and Lift",
        "Computed via momentum exchange method: for each solid \
         boundary node, sum the momentum transferred from adjacent \
         fluid nodes through the lattice velocities. \
         Cd = 2*F_drag / (rho * u^2 * A).",
    );
}

/// Menu screen overlay.
fn menu_ui_system(mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if !ctx.content_rect().is_finite() {
        return;
    }

    bevy_egui::egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("Gororoba: Fluid Dynamics");
            ui.add_space(20.0);
            ui.label("Design a vehicle and test it in a wind tunnel.");
            ui.add_space(40.0);
            ui.label("Press SPACE to start");
            ui.add_space(20.0);
            ui.label("Controls:");
            ui.label("  Right-click + drag: orbit camera");
            ui.label("  Scroll: zoom");
            ui.label("  F1: toggle HUD");
            ui.label("  F2: toggle pedagogy panel");
            ui.label("  Enter: advance to next phase");
            ui.label("  Escape: pause");
        });
    });
}

/// Vehicle design mode UI: preset selection and voxel info.
fn design_ui_system(
    mut contexts: EguiContexts,
    mut config: ResMut<WindTunnelConfig>,
    mut commands: Commands,
    vehicle_query: Query<(Entity, &Vehicle, &VoxelGrid)>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if !ctx.content_rect().is_finite() {
        return;
    }

    bevy_egui::egui::Window::new("Vehicle Designer")
        .anchor(
            bevy_egui::egui::Align2::LEFT_TOP,
            bevy_egui::egui::vec2(10.0, 10.0),
        )
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Shape Presets");
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(config.selected_preset == VehiclePreset::Sphere, "Sphere")
                    .on_hover_text("Sphere: high drag baseline.\nSymmetric bluff body, Cd ~ 0.47.")
                    .clicked()
                {
                    config.selected_preset = VehiclePreset::Sphere;
                    replace_vehicle(&mut commands, &vehicle_query, &config);
                }
                if ui
                    .selectable_label(config.selected_preset == VehiclePreset::Wedge, "Wedge")
                    .on_hover_text(
                        "Wedge: moderate streamlining.\n\
                         Triangular cross-section tapering into the flow.",
                    )
                    .clicked()
                {
                    config.selected_preset = VehiclePreset::Wedge;
                    replace_vehicle(&mut commands, &vehicle_query, &config);
                }
                if ui
                    .selectable_label(config.selected_preset == VehiclePreset::Airfoil, "Airfoil")
                    .on_hover_text(
                        "Airfoil: low drag, high lift.\n\
                         NACA-like elliptical cross-section\n\
                         with tapered trailing edge.",
                    )
                    .clicked()
                {
                    config.selected_preset = VehiclePreset::Airfoil;
                    replace_vehicle(&mut commands, &vehicle_query, &config);
                }
            });

            ui.separator();

            if let Some((_, vehicle, grid)) = vehicle_query.iter().next() {
                if let Some(preset) = vehicle.preset {
                    ui.label(format!("Active preset: {preset:?}"));
                }
                ui.label(format!("Solid cells: {}", grid.solid_count()));
                ui.label(format!("Domain: {}x{}x{}", grid.nx, grid.ny, grid.nz));
            }

            ui.separator();
            ui.heading("Simulation Parameters");

            ui.horizontal(|ui| {
                ui.label("Tau:").on_hover_text(
                    "Relaxation time (BGK collision operator).\n\
                     Controls fluid viscosity: nu = (tau - 0.5) / 3.\n\
                     Lower tau = less viscous (turbulent).\n\
                     Higher tau = more viscous (laminar).\n\
                     Must be > 0.5 for stability.",
                );
                let mut tau_f32 = config.tau as f32;
                ui.add(bevy_egui::egui::Slider::new(&mut tau_f32, 0.51..=2.0));
                config.tau = tau_f32 as f64;
            });

            ui.horizontal(|ui| {
                ui.label("Velocity:").on_hover_text(
                    "Freestream velocity (lattice units).\n\
                     The incoming flow speed from the left.\n\
                     Higher values = faster flow, higher Re.\n\
                     Keep below ~0.1 for LBM stability\n\
                     (Mach number < 0.3).",
                );
                let mut vel = config.freestream_velocity[0] as f32;
                ui.add(bevy_egui::egui::Slider::new(&mut vel, 0.001..=0.15));
                config.freestream_velocity[0] = vel as f64;
            });

            ui.horizontal(|ui| {
                ui.label("Substeps:").on_hover_text(
                    "LBM iterations per frame update.\n\
                     More substeps = faster simulation\n\
                     progress but higher CPU cost per frame.\n\
                     1 = real-time, 30 = 30x accelerated.",
                );
                let mut sub = config.substeps as i32;
                ui.add(bevy_egui::egui::Slider::new(&mut sub, 1..=30));
                config.substeps = sub as usize;
            });

            ui.add_space(10.0);
            ui.label("Press ENTER to start wind tunnel");
        });
}

fn replace_vehicle(
    commands: &mut Commands,
    vehicle_query: &Query<(Entity, &Vehicle, &VoxelGrid)>,
    config: &WindTunnelConfig,
) {
    // Despawn existing vehicle.
    for (entity, _, _) in vehicle_query.iter() {
        commands.entity(entity).despawn();
    }
    // Spawn new vehicle with selected preset.
    let voxels = preset_voxels(config.selected_preset, config.nx, config.ny, config.nz);
    commands.spawn((
        Vehicle {
            preset: Some(config.selected_preset),
        },
        voxels,
    ));
}

/// Wind tunnel running UI: show simulation status.
fn tunnel_ui_system(
    mut contexts: EguiContexts,
    diag_query: Query<&SimulationDiagnostics, With<WindTunnelDomain>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if !ctx.content_rect().is_finite() {
        return;
    }

    bevy_egui::egui::Window::new("Wind Tunnel")
        .anchor(
            bevy_egui::egui::Align2::LEFT_TOP,
            bevy_egui::egui::vec2(10.0, 10.0),
        )
        .resizable(false)
        .show(ctx, |ui| {
            if let Some(diag) = diag_query.iter().next() {
                ui.label(format!("Timestep: {}", diag.timestep));
                if diag.stable {
                    ui.colored_label(bevy_egui::egui::Color32::GREEN, "Simulation stable");
                } else {
                    ui.colored_label(
                        bevy_egui::egui::Color32::RED,
                        "SIMULATION UNSTABLE - reduce velocity or increase tau",
                    );
                }
            } else {
                ui.label("Setting up simulation...");
            }
            ui.add_space(10.0);
            ui.label("Press ENTER for results");
        });
}

/// Results screen: show final aerodynamic coefficients.
fn results_ui_system(mut contexts: EguiContexts, results: Res<AerodynamicResults>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if !ctx.content_rect().is_finite() {
        return;
    }

    bevy_egui::egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.heading("Wind Tunnel Results");
            ui.add_space(20.0);

            bevy_egui::egui::Grid::new("results_grid")
                .num_columns(2)
                .spacing([40.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Drag Coefficient (Cd):");
                    ui.label(format!("{:.4}", results.drag_coefficient));
                    ui.end_row();

                    ui.label("Lift Coefficient (Cl):");
                    ui.label(format!("{:.4}", results.lift_coefficient));
                    ui.end_row();

                    ui.label("Reynolds Number:");
                    ui.label(format!("{:.0}", results.reynolds_number));
                    ui.end_row();

                    ui.label("Final Timestep:");
                    ui.label(format!("{}", results.timestep));
                    ui.end_row();

                    ui.label("Performance (MLUPS):");
                    ui.label(format!("{:.1}", results.mlups));
                    ui.end_row();
                });

            ui.add_space(30.0);
            ui.label("Press ENTER to return to menu");
        });
    });
}

/// Visualize the velocity field as colored gizmo lines.
///
/// Samples the velocity field from the LBM engine and draws
/// direction arrows colored by speed magnitude.
fn velocity_gizmo_system(
    engine: Res<LbmCpuEngine>,
    domain_query: Query<(Entity, &VoxelGrid), With<WindTunnelDomain>>,
    mut gizmos: Gizmos,
) {
    for (entity, grid) in &domain_query {
        if let Some(inst) = engine.get(entity) {
            // Sample every 4th cell to avoid gizmo overload.
            let step = 4;
            for z in (0..grid.nz).step_by(step) {
                for y in (0..grid.ny).step_by(step) {
                    for x in (0..grid.nx).step_by(step) {
                        if grid.get(x, y, z) {
                            continue; // Skip solid cells.
                        }

                        let (_, u) = inst.get_macroscopic(x, y, z);
                        let speed = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();

                        if speed < 1e-6 {
                            continue;
                        }

                        let pos = Vec3::new(
                            x as f32 - grid.nx as f32 / 2.0,
                            y as f32 - grid.ny as f32 / 2.0,
                            z as f32 - grid.nz as f32 / 2.0,
                        );

                        let dir = Vec3::new(u[0], u[1], u[2]).normalize() * (speed * 50.0).min(2.0);

                        // Color: blue (slow) -> red (fast).
                        let t = (speed / 0.1).clamp(0.0, 1.0);
                        let color = Color::srgb(t, 0.2, 1.0 - t);

                        gizmos.line(pos, pos + dir, color);
                    }
                }
            }
        }
    }
}
