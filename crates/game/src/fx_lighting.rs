//! Light on lit FXR particles (#324), after the game's effect shaders.
//!
//! The game lights a lit billboard per vertex in its geometry shader
//! (`GXFfxtessellateSoftTexture.gpo`, the same body in the `.dpo` domain shaders) and finishes in
//! the pixel shader (`GXFfxtessellateBlendSoftTexture.ppo`), both disassembled with
//! `d3dcompiler_47`:
//!
//! - Vertex (`TEXCOORD6`): the irradiance volume's constant band (`g_IrradianceVolumeW`, decoded
//!   `2 (rgb - 0.5) 2^(20 (a - 0.5))`, clamped at 0, times `g_ds_irradianceVolumeDiffuseColorScale`,
//!   volumes A and B blended by `g_ds_irradianceVolumeInterpRatio`) plus every tile point and spot
//!   light at the vertex: `Diff.rgb * saturate(1 - d^2 / R^2) / (1 + 0.7 (d b)^2) *
//!   (0.75 t^2 + 0.25)` with `t = 1.4 / (1.4 + d b)`. That is the deferred point light's wrap
//!   term with no normal (N.L averaged to 1/4), see docs/MAP-FX.md.
//! - Pixel, when `g_ps_bEnableLighting`: `saturate(texture x colour) x (sun x shadow + vertex)`,
//!   the sun being the light set's directional colour 0 (`FC_DirLight[envCbIndex].Col_0`) times
//!   `g_ps_envCbColScale` and a light-mask factor. There is no N.L: a billboard takes the sun
//!   whole.
//!
//! Here it is evaluated on the CPU once per particle (the game does it per vertex, which is the
//! same for a sprite small against the light's range). Inferred or open: the light mask,
//! `g_ps_envCbColScale` and the volume colour scale are taken as 1 (their CPU uploads are not
//! traced); the sun is not shadowed (the game's `g_shadowVolumeTexture` x
//! `g_ps_BillboardShadowIntensity` is not ported); overlapping volumes blend by their edge fade as
//! the map shader's `ambient_light` does, not by the game's A/B ratio; the GS's upper-hemisphere
//! branch (`g_ds_isIVUpper`, which adds the Y band) is not taken; spot lights are left out.

use bevy::prelude::*;

/// The native irradiance volumes' constant (W) band, kept on the CPU for particle lighting.
#[derive(Resource, Default)]
pub struct IrradianceCpu {
    pub volumes: Vec<GiVolume>,
}

/// One volume: world to native UV box space, its world bounds, the record's full voxel size
/// (for the edge fade) and the W plane's size and RGBA8 voxels (x fastest, then y, then z).
pub struct GiVolume {
    pub uv: Mat4,
    pub min: Vec3,
    pub max: Vec3,
    pub dims: UVec3,
    pub w_dims: UVec3,
    pub w: Vec<u32>,
}

impl GiVolume {
    fn texel(&self, x: u32, y: u32, z: u32) -> Vec4 {
        let d = self.w_dims;
        let i = ((z * d.y + y) * d.x + x) as usize;
        let b = self.w.get(i).copied().unwrap_or(0x8080_8080).to_le_bytes();
        Vec4::new(b[0] as f32, b[1] as f32, b[2] as f32, b[3] as f32) / 255.
    }

    /// Trilinear filtered RGBA8 sample at texel coordinates `p` (centres at i + 0.5, clamped as
    /// `gi_sample` in `map_fog_surface.wesl` does), decoded after filtering as the GPU does.
    fn sample(&self, uv: Vec3) -> Vec3 {
        let d = self.w_dims.as_vec3();
        let p = (uv * d).clamp(Vec3::splat(0.5), d - 0.5) - 0.5;
        let i = p.floor();
        let f = p - i;
        let i = i.as_uvec3();
        let hi = (i + 1).min(self.w_dims - 1);
        let mut acc = Vec4::ZERO;
        for (dz, wz) in [(i.z, 1. - f.z), (hi.z, f.z)] {
            for (dy, wy) in [(i.y, 1. - f.y), (hi.y, f.y)] {
                for (dx, wx) in [(i.x, 1. - f.x), (hi.x, f.x)] {
                    acc += self.texel(dx, dy, dz) * (wx * wy * wz);
                }
            }
        }
        (2. * (acc.xyz() - 0.5) * (20. * (acc.w - 0.5)).exp2()).max(Vec3::ZERO)
    }
}

impl IrradianceCpu {
    /// Native irradiance at `pos` (the constant band), 0 outside every volume.
    pub fn at(&self, pos: Vec3) -> Vec3 {
        let mut result = Vec3::ZERO;
        for v in &self.volumes {
            if pos.cmplt(v.min).any() || pos.cmpgt(v.max).any() {
                continue;
            }
            let uv = v.uv.transform_point3(pos);
            let edge = uv.min(1. - uv) * v.dims.as_vec3();
            let x = edge.min_element().clamp(0., 1.);
            let fade = x * x * (3. - 2. * x);
            if fade <= 0. {
                continue;
            }
            result = result.lerp(v.sample(uv), fade);
        }
        result
    }
}

/// One local light as the particle shader reads it, in native units.
#[derive(Clone, Copy, Debug)]
pub struct NativeLight {
    pub position: Vec3,
    pub color: Vec3,
    pub range: f32,
    /// `b`: 1 / the falloff distance.
    pub b: f32,
}

impl NativeLight {
    /// The geometry shader's point light at distance `d`, with no normal.
    pub fn at(&self, d: f32) -> Vec3 {
        let fall = (1. - (d / self.range).powi(2)).clamp(0., 1.);
        if fall <= 0. {
            return Vec3::ZERO;
        }
        let db = d * self.b;
        let t = 1.4 / (1.4 + db);
        self.color * (fall / (1. + 0.7 * db * db) * (0.75 * t * t + 0.25))
    }
}

/// What lights a lit particle this frame, in native units (multiply by
/// [`crate::lighting::ENV_PER_UNIT`] for Bevy).
#[derive(Default, Clone)]
pub struct ParticleLighting {
    /// The light set's directional light 0 diffuse colour times its W.
    pub sun: Vec3,
    pub lights: Vec<NativeLight>,
}

impl ParticleLighting {
    /// The native light on a lit particle at `pos`.
    pub fn at(&self, pos: Vec3, gi: Option<&IrradianceCpu>) -> Vec3 {
        let mut l = self.sun + gi.map_or(Vec3::ZERO, |g| g.at(pos));
        for light in &self.lights {
            l += light.at(light.position.distance(pos));
        }
        l
    }
}

/// Gathers this frame's [`ParticleLighting`] from the light set in force and the point lights
/// (map BTL and FXR 609, both Bevy `PointLight`s in native-scaled units).
pub fn gather(
    lighting: Option<&crate::lighting::MapLighting>,
    points: impl Iterator<Item = (PointLight, Vec3)>,
) -> ParticleLighting {
    use crate::lighting::{LOCAL_LUMENS_PER_UNIT, SCENE_TO_BEVY};
    let sun = lighting.map_or(Vec3::ZERO, |l| {
        l.surface_light(None).direct[1].xyz() / SCENE_TO_BEVY
    });
    let lights = points
        .filter(|(p, _)| p.intensity > 0. && p.range > 0.)
        .map(|(p, position)| {
            // As `native_clustered_light` in map_material.wesl reads a clustered light.
            let scale = crate::map::native_point_light_scale()
                * crate::map::clustered_light_residual(p.radius);
            let falloff = if p.radius > 0. { p.radius } else { p.range };
            let c = p.color.to_linear();
            NativeLight {
                position,
                color: Vec3::new(c.red, c.green, c.blue) * p.intensity / LOCAL_LUMENS_PER_UNIT
                    * scale,
                range: p.range,
                b: 1. / falloff,
            }
        })
        .collect();
    ParticleLighting { sun, lights }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_light_wrap_without_normal() {
        let l = NativeLight {
            position: Vec3::ZERO,
            color: Vec3::ONE,
            range: 5.,
            b: 1.,
        };
        // At the centre: fall 1, t 1, so the full colour.
        assert!((l.at(0.).x - 1.).abs() < 1e-6);
        // Past the range: nothing.
        assert_eq!(l.at(5.), Vec3::ZERO);
        // At 1 m: (1 - 1/25) / 1.7 x (0.75 (1.4/2.4)^2 + 0.25).
        let t: f32 = 1.4 / 2.4;
        let want = 0.96 / 1.7 * (0.75 * t * t + 0.25);
        assert!((l.at(1.).x - want).abs() < 1e-6);
    }

    #[test]
    fn irradiance_decodes_a_flat_volume() {
        // rgb 0.75, a 0.5 + 1/20 (so 2^1): 2 x 0.25 x 2 = 1.
        let a = ((0.5 + 0.05) * 255f32).round() as u32;
        let rgb = (0.75 * 255f32).round() as u32;
        let texel = rgb | rgb << 8 | rgb << 16 | a << 24;
        let gi = IrradianceCpu {
            volumes: vec![GiVolume {
                uv: Mat4::from_scale(Vec3::splat(0.1)),
                min: Vec3::ZERO,
                max: Vec3::splat(10.),
                dims: UVec3::splat(4),
                w_dims: UVec3::splat(4),
                w: vec![texel; 64],
            }],
        };
        let v = gi.at(Vec3::splat(5.));
        assert!((v.x - 1.).abs() < 0.03, "{v}");
        assert_eq!(gi.at(Vec3::splat(11.)), Vec3::ZERO);
    }
}
